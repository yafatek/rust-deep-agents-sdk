//! Procedural macros for Rust Deep Agents SDK
//!
//! This crate provides the `#[tool]` macro that converts regular Rust functions
//! into AI agent tools with automatic JSON Schema generation.

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, FnArg, ItemFn, LitStr, Pat, Type};

/// Converts a Rust function into an AI agent tool with automatic schema generation.
///
/// # Examples
///
/// ```rust
/// use agents_macros::tool;
///
/// #[tool("Greets a person by name")]
/// fn greet(name: String) -> String {
///     format!("Hello, {}!", name)
/// }
///
/// #[tool("Searches the web for information")]
/// async fn web_search(query: String, max_results: Option<u32>) -> Vec<String> {
///     // Implementation
///     vec![]
/// }
/// ```
#[proc_macro_attribute]
pub fn tool(attr: TokenStream, item: TokenStream) -> TokenStream {
    let description = parse_macro_input!(attr as LitStr);
    let input_fn = parse_macro_input!(item as ItemFn);

    let fn_name = &input_fn.sig.ident;
    let fn_name_str = fn_name.to_string();
    let description_str = description.value();
    let is_async = input_fn.sig.asyncness.is_some();

    // Extract parameters
    let mut param_schemas = Vec::new();
    let mut param_idents = Vec::new();
    let mut required_params = Vec::new();
    let mut param_extractions = Vec::new();

    for input in &input_fn.sig.inputs {
        if let FnArg::Typed(pat_type) = input {
            if let Pat::Ident(pat_ident) = &*pat_type.pat {
                let param_name = pat_ident.ident.to_string();
                let param_ident = &pat_ident.ident;
                let param_type = &*pat_type.ty;

                // Check if it's Option<T> (optional parameter)
                let is_optional = is_option_type(param_type);

                if !is_optional {
                    required_params.push(param_name.clone());
                }

                param_idents.push(param_ident.clone());

                // Generate schema for this parameter
                let schema_gen = generate_param_schema(&param_name, param_type, is_optional);
                param_schemas.push(quote! {
                    properties.insert(
                        #param_name.to_string(),
                        #schema_gen
                    );
                });

                // Generate extraction code
                let extraction = generate_param_extraction(&param_name, param_type, is_optional);
                param_extractions.push(extraction);
            }
        }
    }

    // Generate the tool wrapper
    let tool_struct_name = syn::Ident::new(
        &format!("{}Tool", to_pascal_case(&fn_name_str)),
        fn_name.span(),
    );

    let execute_body = if is_async {
        quote! {
            let result = #fn_name(#(#param_idents),*).await;
            let output = serde_json::to_string(&result)
                .unwrap_or_else(|_| format!("{:?}", result));
            Ok(::agents_core::tools::ToolResult::text(&ctx, output))
        }
    } else {
        quote! {
            let result = #fn_name(#(#param_idents),*);
            let output = serde_json::to_string(&result)
                .unwrap_or_else(|_| format!("{:?}", result));
            Ok(::agents_core::tools::ToolResult::text(&ctx, output))
        }
    };

    let expanded = quote! {
        #input_fn

        pub struct #tool_struct_name;

        impl #tool_struct_name {
            pub fn as_tool() -> ::std::sync::Arc<dyn ::agents_core::tools::Tool> {
                ::std::sync::Arc::new(#tool_struct_name)
            }
        }

        #[::async_trait::async_trait]
        impl ::agents_core::tools::Tool for #tool_struct_name {
            fn schema(&self) -> ::agents_core::tools::ToolSchema {
                use ::std::collections::HashMap;
                use ::agents_core::tools::{ToolSchema, ToolParameterSchema};

                let mut properties = HashMap::new();
                #(#param_schemas)*

                ToolSchema::new(
                    #fn_name_str,
                    #description_str,
                    ToolParameterSchema::object(
                        concat!(#fn_name_str, " parameters"),
                        properties,
                        vec![#(#required_params.to_string()),*],
                    ),
                )
            }

            async fn execute(
                &self,
                args: ::serde_json::Value,
                ctx: ::agents_core::tools::ToolContext,
            ) -> ::anyhow::Result<::agents_core::tools::ToolResult> {
                #(#param_extractions)*
                #execute_body
            }
        }
    };

    TokenStream::from(expanded)
}

fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}

fn generate_param_schema(
    param_name: &str,
    param_type: &Type,
    is_optional: bool,
) -> proc_macro2::TokenStream {
    let description = format!("Parameter: {}", param_name);

    // Extract the inner type if it's Option<T>
    let inner_type = if is_optional {
        extract_option_inner_type(param_type)
    } else {
        param_type
    };

    // Generate schema based on type
    match type_to_string(inner_type).as_str() {
        "String" | "str" => quote! {
            ::agents_core::tools::ToolParameterSchema::string(#description)
        },
        "i32" | "i64" | "u32" | "u64" | "isize" | "usize" => quote! {
            ::agents_core::tools::ToolParameterSchema::integer(#description)
        },
        "f32" | "f64" => quote! {
            ::agents_core::tools::ToolParameterSchema::number(#description)
        },
        "bool" => quote! {
            ::agents_core::tools::ToolParameterSchema::boolean(#description)
        },
        _ => {
            // For complex types (Vec, custom structs), default to string
            quote! {
                ::agents_core::tools::ToolParameterSchema::string(#description)
            }
        }
    }
}

fn generate_param_extraction(
    param_name: &str,
    param_type: &Type,
    is_optional: bool,
) -> proc_macro2::TokenStream {
    let param_ident = syn::Ident::new(param_name, proc_macro2::Span::call_site());

    if is_optional {
        let inner_type = extract_option_inner_type(param_type);
        let conversion = generate_type_conversion(inner_type);
        quote! {
            let #param_ident: Option<_> = args.get(#param_name)
                .and_then(|v| #conversion);
        }
    } else {
        let conversion = generate_type_conversion(param_type);
        quote! {
            let #param_ident: #param_type = args.get(#param_name)
                .and_then(|v| #conversion)
                .ok_or_else(|| ::anyhow::anyhow!(concat!("Missing required parameter: ", #param_name)))?;
        }
    }
}

fn generate_type_conversion(ty: &Type) -> proc_macro2::TokenStream {
    match type_to_string(ty).as_str() {
        "String" => quote! { Some(v.as_str()?.to_string()) },
        "str" => quote! { Some(v.as_str()?.to_string()) },
        "i32" | "i64" => quote! { Some(v.as_i64()? as _) },
        "u32" | "u64" => quote! { Some(v.as_u64()? as _) },
        "f32" | "f64" => quote! { Some(v.as_f64()? as _) },
        "bool" => quote! { v.as_bool() },
        _ => quote! { ::serde_json::from_value(v.clone()).ok() },
    }
}

fn extract_option_inner_type(ty: &Type) -> &Type {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner)) = args.args.first() {
                        return inner;
                    }
                }
            }
        }
    }
    ty
}

fn type_to_string(ty: &Type) -> String {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident.to_string();
        }
    }
    "Unknown".to_string()
}

fn to_pascal_case(s: &str) -> String {
    s.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
            }
        })
        .collect()
}
