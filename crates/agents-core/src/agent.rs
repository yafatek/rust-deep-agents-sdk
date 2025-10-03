use async_trait::async_trait;
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::pin::Pin;
use std::sync::Arc;

use crate::llm::StreamChunk;
use crate::messaging::AgentMessage;
use crate::state::AgentStateSnapshot;

/// Planner interface responsible for deciding which actions to take.
#[async_trait]
pub trait PlannerHandle: Send + Sync + std::any::Any {
    async fn plan(
        &self,
        context: PlannerContext,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<PlannerDecision>;

    /// Enable downcasting to concrete types
    fn as_any(&self) -> &dyn std::any::Any;
}

/// Minimal metadata about an agent instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentDescriptor {
    pub name: String,
    pub version: String,
    pub description: Option<String>,
}

/// Message that returns the planner's decision for the next step.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerDecision {
    pub next_action: PlannerAction,
}

/// High-level actions a planner can request from the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PlannerAction {
    CallTool {
        tool_name: String,
        payload: serde_json::Value,
    },
    Respond {
        message: AgentMessage,
    },
    Terminate,
}

/// Context passed to planners containing the latest exchange history.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlannerContext {
    pub history: Vec<AgentMessage>,
    pub system_prompt: String,
    #[serde(default)]
    pub tools: Vec<crate::tools::ToolSchema>,
}

/// Type alias for a stream of agent response chunks
pub type AgentStream = Pin<Box<dyn Stream<Item = anyhow::Result<StreamChunk>> + Send>>;

/// Abstraction for hosting a fully configured agent (planner + tools + prompts).
#[async_trait]
pub trait AgentHandle: Send + Sync {
    async fn describe(&self) -> AgentDescriptor;

    async fn handle_message(
        &self,
        input: AgentMessage,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage>;

    /// Handle a message with streaming response
    /// Default implementation falls back to non-streaming handle_message()
    async fn handle_message_stream(
        &self,
        input: AgentMessage,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentStream> {
        // Default: call non-streaming and wrap result
        let response = self.handle_message(input, state).await?;
        Ok(Box::pin(futures::stream::once(async move {
            Ok(StreamChunk::Done { message: response })
        })))
    }

    /// Get the current pending interrupt if any
    /// Returns None if no interrupts are pending
    async fn current_interrupt(&self) -> anyhow::Result<Option<crate::hitl::AgentInterrupt>> {
        // Default implementation returns None
        Ok(None)
    }

    /// Resume execution after human approval of an interrupt
    ///
    /// # Arguments
    /// * `action` - The human's decision (Accept, Edit, Reject, or Respond)
    ///
    /// # Returns
    /// The agent's response after processing the action
    async fn resume_with_approval(
        &self,
        _action: crate::hitl::HitlAction,
    ) -> anyhow::Result<AgentMessage> {
        // Default implementation returns an error
        anyhow::bail!("resume_with_approval not implemented for this agent")
    }
}

// ToolResponse has been removed - use ToolResult from crate::tools instead
