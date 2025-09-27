use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::command::Command;
use crate::messaging::{AgentMessage, ToolInvocation};
use crate::state::AgentStateSnapshot;

/// Describes a tool that can be invoked by an agent at runtime.
#[async_trait]
pub trait ToolHandle: Send + Sync {
    /// Returns the unique, stable name for this tool.
    fn name(&self) -> &str;

    /// Executes the tool given the invocation payload.
    async fn invoke(&self, invocation: ToolInvocation) -> anyhow::Result<ToolResponse>;
}

/// Planner interface responsible for deciding which actions to take.
#[async_trait]
pub trait PlannerHandle: Send + Sync {
    async fn plan(
        &self,
        context: PlannerContext,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<PlannerDecision>;
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
}

/// Abstraction for hosting a fully configured agent (planner + tools + prompts).
#[async_trait]
pub trait AgentHandle: Send + Sync {
    async fn describe(&self) -> AgentDescriptor;

    async fn handle_message(
        &self,
        input: AgentMessage,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentMessage>;
}

#[derive(Debug, Clone)]
pub enum ToolResponse {
    Message(AgentMessage),
    Command(Command),
}
