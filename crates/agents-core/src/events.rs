//! Event system for agent lifecycle tracking and progress broadcasting

use crate::state::TodoItem;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum AgentEvent {
    AgentStarted(AgentStartedEvent),
    AgentCompleted(AgentCompletedEvent),
    ToolStarted(ToolStartedEvent),
    ToolCompleted(ToolCompletedEvent),
    ToolFailed(ToolFailedEvent),
    SubAgentStarted(SubAgentStartedEvent),
    SubAgentCompleted(SubAgentCompletedEvent),
    TodosUpdated(TodosUpdatedEvent),
    StateCheckpointed(StateCheckpointedEvent),
    PlanningComplete(PlanningCompleteEvent),
}

impl AgentEvent {
    pub fn event_type_name(&self) -> &'static str {
        match self {
            AgentEvent::AgentStarted(_) => "agent_started",
            AgentEvent::AgentCompleted(_) => "agent_completed",
            AgentEvent::ToolStarted(_) => "tool_started",
            AgentEvent::ToolCompleted(_) => "tool_completed",
            AgentEvent::ToolFailed(_) => "tool_failed",
            AgentEvent::SubAgentStarted(_) => "sub_agent_started",
            AgentEvent::SubAgentCompleted(_) => "sub_agent_completed",
            AgentEvent::TodosUpdated(_) => "todos_updated",
            AgentEvent::StateCheckpointed(_) => "state_checkpointed",
            AgentEvent::PlanningComplete(_) => "planning_complete",
        }
    }

    pub fn metadata(&self) -> &EventMetadata {
        match self {
            AgentEvent::AgentStarted(e) => &e.metadata,
            AgentEvent::AgentCompleted(e) => &e.metadata,
            AgentEvent::ToolStarted(e) => &e.metadata,
            AgentEvent::ToolCompleted(e) => &e.metadata,
            AgentEvent::ToolFailed(e) => &e.metadata,
            AgentEvent::SubAgentStarted(e) => &e.metadata,
            AgentEvent::SubAgentCompleted(e) => &e.metadata,
            AgentEvent::TodosUpdated(e) => &e.metadata,
            AgentEvent::StateCheckpointed(e) => &e.metadata,
            AgentEvent::PlanningComplete(e) => &e.metadata,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventMetadata {
    pub thread_id: String,
    pub correlation_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub customer_id: Option<String>,
    pub timestamp: String,
}

impl EventMetadata {
    pub fn new(thread_id: String, correlation_id: String, customer_id: Option<String>) -> Self {
        Self {
            thread_id,
            correlation_id,
            customer_id,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentStartedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub message_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentCompletedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub duration_ms: u64,
    pub response_preview: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolStartedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub input_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCompletedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub duration_ms: u64,
    pub result_summary: String,
    pub success: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolFailedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub duration_ms: u64,
    pub error_message: String,
    pub is_recoverable: bool,
    pub retry_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentStartedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub instruction_summary: String,
    pub delegation_depth: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubAgentCompletedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub duration_ms: u64,
    pub result_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodosUpdatedEvent {
    pub metadata: EventMetadata,
    pub todos: Vec<TodoItem>,
    pub pending_count: usize,
    pub in_progress_count: usize,
    pub completed_count: usize,
    pub last_updated: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateCheckpointedEvent {
    pub metadata: EventMetadata,
    pub checkpoint_id: String,
    pub state_size_bytes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlanningCompleteEvent {
    pub metadata: EventMetadata,
    pub action_type: String,
    pub action_summary: String,
}

#[async_trait]
pub trait EventBroadcaster: Send + Sync {
    fn id(&self) -> &str;
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()>;
    fn should_broadcast(&self, _event: &AgentEvent) -> bool {
        true
    }
}

pub struct EventDispatcher {
    broadcasters: Vec<Arc<dyn EventBroadcaster>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            broadcasters: Vec::new(),
        }
    }

    pub fn add_broadcaster(&mut self, broadcaster: Arc<dyn EventBroadcaster>) {
        self.broadcasters.push(broadcaster);
    }

    pub async fn dispatch(&self, event: AgentEvent) {
        let broadcasters = self.broadcasters.clone();
        for broadcaster in broadcasters {
            let event_clone = event.clone();
            tokio::spawn(async move {
                if broadcaster.should_broadcast(&event_clone) {
                    if let Err(e) = broadcaster.broadcast(&event_clone).await {
                        tracing::warn!(
                            broadcaster_id = broadcaster.id(),
                            error = %e,
                            "Failed to broadcast event"
                        );
                    }
                }
            });
        }
    }
}

impl Default for EventDispatcher {
    fn default() -> Self {
        Self::new()
    }
}
