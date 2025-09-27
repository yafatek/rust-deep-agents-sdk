//! Toolkit of default tools and helpers mirroring the Python reference implementation.
//! Includes filesystem manipulation tools, todo list management, and planning scaffolds.

pub mod filesystem;
pub mod todos;
mod search;

pub use filesystem::{EditFileTool, LsTool, ReadFileTool, WriteFileTool};
pub use todos::WriteTodosTool;

use agents_core::agent::ToolResponse;
use agents_core::messaging::{
    AgentMessage, MessageContent, MessageMetadata, MessageRole, ToolInvocation,
};

pub(crate) fn metadata_from(invocation: &ToolInvocation) -> Option<MessageMetadata> {
    invocation.tool_call_id.as_ref().map(|id| MessageMetadata {
        tool_call_id: Some(id.clone()),
    })
}

pub(crate) fn tool_text_response(
    invocation: &ToolInvocation,
    message: impl Into<String>,
) -> ToolResponse {
    ToolResponse::Message(AgentMessage {
        role: MessageRole::Tool,
        content: MessageContent::Text(message.into()),
        metadata: metadata_from(invocation),
    })
}
