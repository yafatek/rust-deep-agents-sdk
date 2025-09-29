//! Helper functions for creating user tools from regular Rust functions
//! 
//! This module provides utilities to convert regular Rust functions into ToolHandle
//! implementations for the `tools` parameter in create_deep_agent(), making it easier
//! to create custom tools while keeping all existing built-in tools unchanged.

use std::sync::Arc;
use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageRole, ToolInvocation};
use async_trait::async_trait;
use serde_json::Value;

pub fn create_tool<F, Fut>(
    name: &'static str,
    description: &'static str,
    handler: F,
) -> Arc<dyn ToolHandle>
where
    F: Fn(Value) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = anyhow::Result<String>> + Send + 'static,
{
    Arc::new(FunctionTool {
        name,
        // description,
        handler: Box::new(move |args| Box::pin(handler(args))),
    })
}

/// Create a tool from a synchronous function

pub fn create_sync_tool<F>(
    name: &'static str,
    // description: &'static str,
    handler: F,
) -> Arc<dyn ToolHandle>
where
    F: Fn(Value) -> anyhow::Result<String> + Send + Sync + 'static,
{
    Arc::new(SyncFunctionTool {
        name,
        // description,
        handler: Box::new(handler),
    })
}

// Internal implementation for async tools
struct FunctionTool {
    name: &'static str,
    // description: &'static str,
    handler: Box<dyn Fn(Value) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<String>> + Send>> + Send + Sync>,
}

#[async_trait]
impl ToolHandle for FunctionTool {
    fn name(&self) -> &str {
        self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let result = (self.handler)(invocation.args).await?;
        
        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(result),
            metadata: invocation.tool_call_id.map(|id| agents_core::messaging::MessageMetadata {
                tool_call_id: Some(id),
                cache_control: None,
            }),
        }))
    }
}

// Internal implementation for sync tools
struct SyncFunctionTool {
    name: &'static str,
    // description: &'static str,
    handler: Box<dyn Fn(Value) -> anyhow::Result<String> + Send + Sync>,
}

#[async_trait]
impl ToolHandle for SyncFunctionTool {
    fn name(&self) -> &str {
        self.name
    }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        let result = (self.handler)(invocation.args)?;
        
        Ok(ToolResponse::Message(AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(result),
            metadata: invocation.tool_call_id.map(|id| agents_core::messaging::MessageMetadata {
                tool_call_id: Some(id),
                cache_control: None,
            }),
        }))
    }
}

/// Macro for creating tools with typed parameters (advanced usage)
#[macro_export]
macro_rules! tool_fn {
    (
        name: $name:expr,
        description: $desc:expr,
        |$($param:ident: $param_type:ty),*| $body:expr
    ) => {
        $crate::create_tool($name, $desc, move |args: serde_json::Value| async move {
            // Extract parameters with proper error handling
            $(
                let $param: $param_type = args.get(stringify!($param))
                    .ok_or_else(|| anyhow::anyhow!("Missing required parameter: {}", stringify!($param)))?
                    .clone()
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid type for parameter: {}", stringify!($param)))?;
            )*
            
            // Call the user's function
            $body.await
        })
    };
}
