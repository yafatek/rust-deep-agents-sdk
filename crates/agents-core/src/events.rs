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
    TokenUsage(TokenUsageEvent),
    StreamingToken(StreamingTokenEvent),
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
            AgentEvent::TokenUsage(_) => "token_usage",
            AgentEvent::StreamingToken(_) => "streaming_token",
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
            AgentEvent::TokenUsage(e) => &e.metadata,
            AgentEvent::StreamingToken(e) => &e.metadata,
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
    pub response_preview: String, // Truncated for logs (~100 chars)
    pub response: String,         // Full response text
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsageEvent {
    pub metadata: EventMetadata,
    pub usage: TokenUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingTokenEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenUsage {
    /// Number of input tokens
    pub input_tokens: u32,
    /// Number of output tokens
    pub output_tokens: u32,
    /// Total tokens used
    pub total_tokens: u32,
    /// Estimated cost in USD
    pub estimated_cost: f64,
    /// Provider name
    pub provider: String,
    /// Model name
    pub model: String,
    /// Request duration in milliseconds
    pub duration_ms: u64,
    /// Timestamp of the request
    pub timestamp: String,
}

impl TokenUsage {
    pub fn new(
        input_tokens: u32,
        output_tokens: u32,
        provider: impl Into<String>,
        model: impl Into<String>,
        duration_ms: u64,
        estimated_cost: f64,
    ) -> Self {
        let provider = provider.into();
        let model = model.into();
        let total_tokens = input_tokens + output_tokens;

        Self {
            input_tokens,
            output_tokens,
            total_tokens,
            estimated_cost,
            provider,
            model,
            duration_ms,
            timestamp: chrono::Utc::now().to_rfc3339(),
        }
    }
}

#[async_trait]
pub trait EventBroadcaster: Send + Sync {
    fn id(&self) -> &str;
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()>;
    fn should_broadcast(&self, _event: &AgentEvent) -> bool {
        true
    }

    /// Indicates whether this broadcaster supports streaming token events.
    /// Default is false for backward compatibility.
    fn supports_streaming(&self) -> bool {
        false
    }
}

pub struct EventDispatcher {
    broadcasters: std::sync::RwLock<Vec<Arc<dyn EventBroadcaster>>>,
}

impl EventDispatcher {
    pub fn new() -> Self {
        Self {
            broadcasters: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Add a broadcaster (supports dynamic addition with interior mutability)
    pub fn add_broadcaster(&self, broadcaster: Arc<dyn EventBroadcaster>) {
        if let Ok(mut broadcasters) = self.broadcasters.write() {
            broadcasters.push(broadcaster);
        } else {
            tracing::error!("Failed to acquire write lock on broadcasters");
        }
    }

    pub async fn dispatch(&self, event: AgentEvent) {
        let broadcasters = {
            if let Ok(guard) = self.broadcasters.read() {
                guard.clone()
            } else {
                tracing::error!("Failed to acquire read lock on broadcasters");
                return;
            }
        };

        for broadcaster in broadcasters {
            let event_clone = event.clone();
            tokio::spawn(async move {
                // Skip streaming tokens for broadcasters that don't support them
                if matches!(event_clone, AgentEvent::StreamingToken(_))
                    && !broadcaster.supports_streaming()
                {
                    return;
                }

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
