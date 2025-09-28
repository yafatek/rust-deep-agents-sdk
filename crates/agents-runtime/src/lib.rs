//! Tokio-powered runtime that glues together planners, tools, and prompt packs.
//! The initial implementation focuses on synchronous message handling with
//! pluggable state stores and tracing hooks.

use std::sync::Arc;

use agents_core::agent::{AgentDescriptor, AgentHandle};
use agents_core::messaging::AgentMessage;
use agents_core::state::AgentStateSnapshot;
use async_trait::async_trait;

pub mod graph;
pub mod middleware;
pub mod planner;
pub mod providers;

// Re-export key functions for convenience
pub use graph::{create_deep_agent, create_deep_agent_from_config, get_default_model};

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
