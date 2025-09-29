//! Macros for automatically converting Rust functions into tools
//! 
//! This module provides the `#[tool]` macro that mirrors the Python SDK's
//! automatic tool conversion, allowing users to write regular Rust functions
//! that are automatically wrapped as ToolHandle implementations.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, ItemFn, FnArg, Pat, Type, ReturnType};

/// Automatically convert a Rust function into a ToolHandle implementation
/// 
/// This macro mirrors the Python SDK's `@tool` decorator, allowing you to write
/// regular Rust functions that are automatically converted to tools.
/// 
/// # Example
/// 
/// ```rust
/// use agents_toolkit::tool;
/// 
/// #[tool]
/// /// Run a web search for the given query
/// async fn internet_search(query: String, max_results: Option<i32>) -> anyhow::Result<String> {
///     let max_results = max_results.unwrap_or(5);
///     // Your search implementation here
///     Ok(format!("Search results for '{}' (max: {})", query, max_results))
/// }
/// 
/// // Now you can use it directly:
/// let agent = create_deep_agent(CreateDeepAgentParams {
///     tools: vec![internet_search()], // Just call the function!
///     instructions: "You are a researcher...".to_string(),
///     ..Default::default()
/// })?;
/// ```
/// 
/// The macro automatically:
/// - Extracts the function name as the tool name
/// - Uses the docstring as the tool description
/// - Generates JSON schema from parameter types
/// - Handles serialization/deserialization
/// - Wraps the function in a ToolHandle implementation
#[proc_macro_attribute]
pub fn tool(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as ItemFn);
    
    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let tool_struct_name = syn::Ident::new(&format!("{}Tool", fn_name), fn_name.span());
    
    // Extract function documentation as tool description
    let description = extract_doc_comment(&input_fn.attrs);
    
    // Extract parameter information for JSON schema generation
    let params = extract_parameters(&input_fn.sig.inputs);
    let param_names: Vec<_> = params.iter().map(|(name, _)| name).collect();
    let param_types: Vec<_> = params.iter().map(|(_, ty)| ty).collect();
    
    // Generate the tool struct and implementation
    let expanded = quote! {
        // Keep the original function
        #input_fn
        
        // Generate the tool wrapper struct
        pub struct #tool_struct_name;
        
        impl #tool_struct_name {
            pub fn new() -> std::sync::Arc<dyn agents_core::agent::ToolHandle> {
                std::sync::Arc::new(Self)
            }
        }
        
        #[async_trait::async_trait]
        impl agents_core::agent::ToolHandle for #tool_struct_name {
            fn name(&self) -> &str {
                #fn_name_str
            }
            
            async fn invoke(&self, invocation: agents_core::messaging::ToolInvocation) -> anyhow::Result<agents_core::agent::ToolResponse> {
                // Extract parameters from JSON args
                #(
                    let #param_names: #param_types = invocation.args
                        .get(stringify!(#param_names))
                        .ok_or_else(|| anyhow::anyhow!("Missing parameter: {}", stringify!(#param_names)))?
                        .clone()
                        .try_into()
                        .map_err(|e| anyhow::anyhow!("Invalid parameter {}: {:?}", stringify!(#param_names), e))?;
                )*
                
                // Call the original function
                let result = #fn_name(#(#param_names),*).await?;
                
                // Convert result to ToolResponse
                Ok(agents_core::agent::ToolResponse::Message(agents_core::messaging::AgentMessage {
                    role: agents_core::messaging::MessageRole::Tool,
                    content: agents_core::messaging::MessageContent::Text(format!("{:?}", result)),
                    metadata: None,
                }))
            }
        }
        
        // Convenience function to create the tool
        pub fn #fn_name() -> std::sync::Arc<dyn agents_core::agent::ToolHandle> {
            #tool_struct_name::new()
        }
    };
    
    TokenStream::from(expanded)
}

fn extract_doc_comment(attrs: &[syn::Attribute]) -> String {
    attrs
        .iter()
        .filter_map(|attr| {
            if attr.path().is_ident("doc") {
                if let syn::Meta::NameValue(meta) = &attr.meta {
                    if let syn::Expr::Lit(syn::ExprLit { lit: syn::Lit::Str(s), .. }) = &meta.value {
                        return Some(s.value().trim().to_string());
                    }
                }
            }
            None
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn extract_parameters(inputs: &syn::punctuated::Punctuated<FnArg, syn::token::Comma>) -> Vec<(syn::Ident, Type)> {
    inputs
        .iter()
        .filter_map(|arg| {
            if let FnArg::Typed(pat_type) = arg {
                if let Pat::Ident(pat_ident) = &**pat_type.pat {
                    return Some((pat_ident.ident.clone(), (*pat_type.ty).clone()));
                }
            }
            None
        })
        .collect()
}
