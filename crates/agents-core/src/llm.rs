use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;

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

/// A chunk of streaming response from the LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamChunk {
    /// A text delta to append to the response
    TextDelta(String),
    /// The stream has finished
    Done {
        /// The complete final message
        message: AgentMessage,
    },
    /// An error occurred during streaming
    Error(String),
}

/// Type alias for a pinned boxed stream of chunks
pub type ChunkStream = Pin<Box<dyn Stream<Item = anyhow::Result<StreamChunk>> + Send>>;

#[async_trait]
pub trait LanguageModel: Send + Sync {
    /// Generate a complete response (non-streaming)
    async fn generate(&self, request: LlmRequest) -> anyhow::Result<LlmResponse>;

    /// Generate a streaming response
    /// Default implementation falls back to non-streaming generate()
    async fn generate_stream(&self, request: LlmRequest) -> anyhow::Result<ChunkStream> {
        // Default implementation: call generate() and return complete response as a single chunk
        let response = self.generate(request).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok(StreamChunk::Done {
                message: response.message,
            })
        })))
    }
}
