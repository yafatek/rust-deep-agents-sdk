//! Tool builder utilities for creating tools from functions
//!
//! This module provides ergonomic helpers for converting regular Rust functions
//! into Tool implementations that can be registered with agents.

use agents_core::tools::{Tool, ToolBox, ToolContext, ToolParameterSchema, ToolResult, ToolSchema};
use async_trait::async_trait;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Type alias for async tool handler functions
pub type AsyncToolFn =
    Arc<dyn Fn(Value, ToolContext) -> BoxFuture<'static, anyhow::Result<ToolResult>> + Send + Sync>;

/// Type alias for sync tool handler functions
pub type SyncToolFn = Arc<dyn Fn(Value, ToolContext) -> anyhow::Result<ToolResult> + Send + Sync>;

/// A tool implementation backed by a function/closure
pub struct FunctionTool {
    schema: ToolSchema,
    handler: AsyncToolFn,
}

#[async_trait]
impl Tool for FunctionTool {
    fn schema(&self) -> ToolSchema {
        self.schema.clone()
    }

    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        (self.handler)(args, ctx).await
    }
}

/// Builder for creating tools from async functions
pub struct ToolBuilder {
    name: String,
    description: String,
    parameters: Option<ToolParameterSchema>,
}

impl ToolBuilder {
    /// Start building a new tool with the given name and description
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
            parameters: None,
        }
    }

    /// Set the parameter schema for this tool
    pub fn with_parameters(mut self, parameters: ToolParameterSchema) -> Self {
        self.parameters = Some(parameters);
        self
    }

    /// Build the tool with an async handler function
    pub fn build_async<F, Fut>(self, handler: F) -> ToolBox
    where
        F: Fn(Value, ToolContext) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = anyhow::Result<ToolResult>> + Send + 'static,
    {
        let schema = ToolSchema::new(
            self.name,
            self.description,
            self.parameters.unwrap_or_else(|| {
                ToolParameterSchema::object("No parameters", Default::default(), Vec::new())
            }),
        );

        let handler: AsyncToolFn = Arc::new(move |args, ctx| Box::pin(handler(args, ctx)));

        Arc::new(FunctionTool { schema, handler })
    }

    /// Build the tool with a sync handler function
    pub fn build_sync<F>(self, handler: F) -> ToolBox
    where
        F: Fn(Value, ToolContext) -> anyhow::Result<ToolResult> + Send + Sync + 'static,
    {
        let handler = Arc::new(handler);
        self.build_async(move |args, ctx| {
            let handler = handler.clone();
            async move { handler(args, ctx) }
        })
    }
}

/// Quick helper to create a simple async tool
pub fn tool<F, Fut>(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: ToolParameterSchema,
    handler: F,
) -> ToolBox
where
    F: Fn(Value, ToolContext) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<ToolResult>> + Send + 'static,
{
    ToolBuilder::new(name, description)
        .with_parameters(parameters)
        .build_async(handler)
}

/// Quick helper to create a simple sync tool
pub fn tool_sync<F>(
    name: impl Into<String>,
    description: impl Into<String>,
    parameters: ToolParameterSchema,
    handler: F,
) -> ToolBox
where
    F: Fn(Value, ToolContext) -> anyhow::Result<ToolResult> + Send + Sync + 'static,
{
    ToolBuilder::new(name, description)
        .with_parameters(parameters)
        .build_sync(handler)
}


#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::state::AgentStateSnapshot;
    use serde_json::json;

    #[tokio::test]
    async fn function_tool_executes_handler() {
        let tool = ToolBuilder::new("echo", "Echoes input")
            .with_parameters(ToolParameterSchema::object(
                "Echo parameters",
                [(
                    "message".to_string(),
                    ToolParameterSchema::string("Message to echo"),
                )]
                .into_iter()
                .collect(),
                vec!["message".to_string()],
            ))
            .build_async(|args, ctx| async move {
                let msg = args["message"].as_str().unwrap_or("empty");
                Ok(ToolResult::text(&ctx, format!("Echo: {}", msg)))
            });

        let schema = tool.schema();
        assert_eq!(schema.name, "echo");
        assert_eq!(schema.description, "Echoes input");

        let ctx = ToolContext::new(Arc::new(AgentStateSnapshot::default()));
        let result = tool
            .execute(json!({"message": "hello"}), ctx)
            .await
            .unwrap();

        match result {
            ToolResult::Message(msg) => {
                assert_eq!(msg.content.as_text().unwrap(), "Echo: hello");
            }
            _ => panic!("Expected message result"),
        }
    }

    #[tokio::test]
    async fn sync_tool_works() {
        let tool = tool_sync(
            "add",
            "Adds two numbers",
            ToolParameterSchema::object(
                "Add parameters",
                [
                    ("a".to_string(), ToolParameterSchema::number("First number")),
                    (
                        "b".to_string(),
                        ToolParameterSchema::number("Second number"),
                    ),
                ]
                .into_iter()
                .collect(),
                vec!["a".to_string(), "b".to_string()],
            ),
            |args, ctx| {
                let a = args["a"].as_f64().unwrap_or(0.0);
                let b = args["b"].as_f64().unwrap_or(0.0);
                let sum = a + b;
                Ok(ToolResult::text(&ctx, format!("Sum: {}", sum)))
            },
        );

        let ctx = ToolContext::new(Arc::new(AgentStateSnapshot::default()));
        let result = tool.execute(json!({"a": 5, "b": 3}), ctx).await.unwrap();

        match result {
            ToolResult::Message(msg) => {
                assert_eq!(msg.content.as_text().unwrap(), "Sum: 8");
            }
            _ => panic!("Expected message result"),
        }
    }
}
