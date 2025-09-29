use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::messaging::AgentMessage;
use crate::tools::ToolSchema;

/// Minimal request structure passed to a language model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    pub system_prompt: String,
    pub messages: Vec<AgentMessage>,
    /// Available tools that the LLM can invoke
    #[serde(default)]
    pub tools: Vec<ToolSchema>,
}

impl LlmRequest {
    /// Create a new LLM request
    pub fn new(system_prompt: impl Into<String>, messages: Vec<AgentMessage>) -> Self {
        Self {
            system_prompt: system_prompt.into(),
            messages,
            tools: Vec::new(),
        }
    }

    /// Add tools to the request
    pub fn with_tools(mut self, tools: Vec<ToolSchema>) -> Self {
        self.tools = tools;
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub message: AgentMessage,
}

#[async_trait]
pub trait LanguageModel: Send + Sync {
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse>;
}
