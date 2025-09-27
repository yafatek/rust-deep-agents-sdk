use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::messaging::ToolInvocation;
use async_trait::async_trait;

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Boxed async function signature for a tool implementation.
pub type BoxedToolFn = Arc<
    dyn Fn(ToolInvocation) -> BoxFuture<'static, anyhow::Result<ToolResponse>> + Send + Sync + 'static,
>;

/// Wrap a plain async function/closure as a ToolHandle.
///
/// Mirrors the Python experience of passing plain functions as tools.
/// Example:
/// let tool = FunctionTool::new(
///     "internet_search",
///     boxed_tool_fn(|inv| async move {
///         // parse inv.args and return a ToolResponse
///         // Ok(ToolResponse::Message(AgentMessage::text("done")))
///         anyhow::bail!("example only")
///     })
/// );
pub struct FunctionTool {
    name: String,
    f: BoxedToolFn,
}

impl FunctionTool {
    /// Create a new FunctionTool from a name and async function/closure.
    pub fn new(name: impl Into<String>, f: BoxedToolFn) -> Self {
        Self { name: name.into(), f }
    }
}

#[async_trait]
impl ToolHandle for FunctionTool {
    fn name(&self) -> &str { &self.name }

    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse> {
        (self.f)(invocation).await
    }
}

/// Helper to box an async closure into the required type.
pub fn boxed_tool_fn<F, Fut>(f: F) -> BoxedToolFn
where
    F: Fn(ToolInvocation) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = anyhow::Result<ToolResponse>> + Send + 'static,
{
    Arc::new(move |invocation| {
        let fut = f(invocation);
        Box::pin(fut) as BoxFuture<'static, _>
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::messaging::{AgentMessage, MessageContent, MessageRole};
    use serde_json::json;

    #[tokio::test]
    async fn function_tool_invokes_closure() {
        let tool = FunctionTool::new(
            "echo",
            boxed_tool_fn(|inv| async move {
                let text = inv.args.get("text").and_then(|v| v.as_str()).unwrap_or("");
                Ok(ToolResponse::Message(AgentMessage {
                    role: MessageRole::Tool,
                    content: MessageContent::Text(format!("echo:{text}")),
                    metadata: None,
                }))
            }),
        );

        let out = tool
            .invoke(agents_core::messaging::ToolInvocation {
                tool_name: "echo".into(),
                args: json!({"text": "hi"}),
                tool_call_id: None,
            })
            .await
            .unwrap();

        match out {
            ToolResponse::Message(m) => match m.content {
                MessageContent::Text(t) => assert_eq!(t, "echo:hi"),
                _ => panic!("expected text"),
            },
            _ => panic!("expected message"),
        }
    }
}

