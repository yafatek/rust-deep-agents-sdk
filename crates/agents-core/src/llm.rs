use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::messaging::AgentMessage;

/// Minimal request structure passed to a language model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub message: AgentMessage,
}

#[async_trait]
pub trait LanguageModel: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse>;
}
