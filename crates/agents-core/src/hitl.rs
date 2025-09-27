use serde::{Deserialize, Serialize};

use crate::messaging::AgentMessage;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HitlAction {
    Approve,
    Reject { reason: Option<String> },
    Respond { message: AgentMessage },
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
