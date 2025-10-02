//! Tokio-powered runtime that glues together planners, tools, and prompt packs.
//! The initial implementation focuses on synchronous message handling with
//! pluggable state stores and tracing hooks.

use std::sync::Arc;

use agents_core::agent::{AgentDescriptor, AgentHandle};
use agents_core::messaging::AgentMessage;
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;

pub mod agent;
pub mod middleware;
pub mod planner;
pub mod prompts;
pub mod providers;

// Re-export key functions for convenience - now from the agent module
pub use agent::{
    create_async_deep_agent, create_deep_agent, get_default_model, ConfigurableAgentBuilder,
    DeepAgent, SubAgentConfig, SummarizationConfig,
};

// Re-export provider configurations and models
pub use providers::{
    AnthropicConfig, AnthropicMessagesModel, GeminiChatModel, GeminiConfig, OpenAiChatModel,
    OpenAiConfig,
};

/// Default runtime wrapper that delegates to an inner agent implementation.
pub struct RuntimeAgent<T>
where
    T: AgentHandle,
{
    inner: Arc<T>,
}

impl<T> RuntimeAgent<T>
where
    T: AgentHandle,
{
    pub fn new(inner: Arc<T>) -> Self {
        Self { inner }
    }
}

#[async_trait]
impl<T> AgentHandle for RuntimeAgent<T>
where
    T: AgentHandle + Sync + Send,
{
    async fn describe(&self) -> AgentDescriptor {
        self.inner.describe().await
    }

    async fn handle_message(
        &self,
        input: AgentMessage,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage> {
        tracing::debug!(role = ?input.role, "handling message");
        self.inner.handle_message(input, state).await
    }
}
