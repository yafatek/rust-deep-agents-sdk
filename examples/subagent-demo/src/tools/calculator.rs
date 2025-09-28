use std::sync::Arc;

use agents_core::agent::{ToolHandle, ToolResponse};
use agents_core::messaging::{AgentMessage, MessageContent, MessageMetadata, MessageRole};
use agents_toolkit::adapters::function_tool::{boxed_tool_fn, FunctionTool};
use anyhow::Context;

pub fn calculator_tool() -> Arc<dyn ToolHandle> {
    let handler = boxed_tool_fn(|invocation| async move {
        let expression = invocation
            .args
            .get("expression")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("calculator requires 'expression' string argument"))?;

        let result = meval::eval_str(expression)
            .with_context(|| format!("failed to evaluate expression '{expression}'"))?;

        let message = AgentMessage {
            role: MessageRole::Tool,
            content: MessageContent::Text(format!("Result: {result}")),
            metadata: Some(MessageMetadata {
                tool_call_id: invocation.tool_call_id.clone(),
                cache_control: None,
            }),
        };

        Ok(ToolResponse::Message(message))
    });

    Arc::new(FunctionTool::new("calculator", handler))
}
