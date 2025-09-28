use serde::{Deserialize, Serialize};

use crate::messaging::AgentMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HitlAction {
    Approve,
    Reject { reason: Option<String> },
    Respond { message: AgentMessage },
    /// Edit the pending tool call by choosing a tool `action` and new `args`.
    Edit { action: String, args: serde_json::Value },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HitlInterrupt {
    pub tool_name: String,
    pub message: AgentMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AgentInterrupt {
    HumanInLoop(HitlInterrupt),
}
