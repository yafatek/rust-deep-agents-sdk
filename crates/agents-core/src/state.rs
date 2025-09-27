use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Snapshot of agent state shared between runtime, planners, and tools.
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AgentStateSnapshot {
    pub todos: Vec<TodoItem>,
    pub files: BTreeMap<String, String>,
    pub scratchpad: BTreeMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
}

impl TodoItem {
    pub fn pending(content: impl Into<String>) -> Self {
        Self {
            content: content.into(),
            status: TodoStatus::Pending,
        }
    }
}
