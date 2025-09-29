//! Persistence traits for checkpointing agent state between runs.

use crate::state::AgentStateSnapshot;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Unique identifier for a conversation thread/session.
pub type ThreadId = String;

/// Configuration for a checkpointer instance.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CheckpointerConfig {
    /// Additional configuration parameters specific to the checkpointer implementation.
    pub params: HashMap<String, serde_json::Value>,
}

/// Trait for persisting and retrieving agent state between conversation runs.
/// This mirrors the LangGraph Checkpointer interface used in the Python implementation.
#[async_trait]
pub trait Checkpointer: Send + Sync {
    /// Save the current agent state for a given thread.
    async fn save_state(
        &self,
        thread_id: &ThreadId,
        state: &AgentStateSnapshot,
    ) -> anyhow::Result<()>;

    /// Load the last saved state for a given thread.
    /// Returns None if no state exists for this thread.
    async fn load_state(&self, thread_id: &ThreadId) -> anyhow::Result<Option<AgentStateSnapshot>>;

    /// Delete all saved state for a given thread.
    async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()>;

    /// List all thread IDs that have saved state.
    async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>>;
}

/// In-memory checkpointer for testing and development.
/// State is not persisted between process restarts.
#[derive(Debug, Default)]
pub struct InMemoryCheckpointer {
    states: std::sync::RwLock<HashMap<ThreadId, AgentStateSnapshot>>,
}

impl InMemoryCheckpointer {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl Checkpointer for InMemoryCheckpointer {
    async fn save_state(
        &self,
        thread_id: &ThreadId,
        state: &AgentStateSnapshot,
    ) -> anyhow::Result<()> {
        let mut states = self.states.write().map_err(|_| {
            anyhow::anyhow!("Failed to acquire write lock on in-memory checkpointer")
        })?;
        states.insert(thread_id.clone(), state.clone());
        tracing::debug!(thread_id = %thread_id, "Saved agent state to memory");
        Ok(())
    }

    async fn load_state(&self, thread_id: &ThreadId) -> anyhow::Result<Option<AgentStateSnapshot>> {
        let states = self.states.read().map_err(|_| {
            anyhow::anyhow!("Failed to acquire read lock on in-memory checkpointer")
        })?;
        let state = states.get(thread_id).cloned();
        if state.is_some() {
            tracing::debug!(thread_id = %thread_id, "Loaded agent state from memory");
        }
        Ok(state)
    }

    async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        let mut states = self.states.write().map_err(|_| {
            anyhow::anyhow!("Failed to acquire write lock on in-memory checkpointer")
        })?;
        states.remove(thread_id);
        tracing::debug!(thread_id = %thread_id, "Deleted thread from memory");
        Ok(())
    }

    async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>> {
        let states = self.states.read().map_err(|_| {
            anyhow::anyhow!("Failed to acquire read lock on in-memory checkpointer")
        })?;
        Ok(states.keys().cloned().collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::{TodoItem, TodoStatus};

    fn sample_state() -> AgentStateSnapshot {
        let mut state = AgentStateSnapshot::default();
        state.todos.push(TodoItem {
            content: "Test todo".to_string(),
            status: TodoStatus::Pending,
        });
        state
            .files
            .insert("test.txt".to_string(), "content".to_string());
        state
            .scratchpad
            .insert("key".to_string(), serde_json::json!("value"));
        state
    }

    #[tokio::test]
    async fn in_memory_checkpointer_save_and_load() {
        let checkpointer = InMemoryCheckpointer::new();
        let thread_id = "test-thread".to_string();
        let state = sample_state();

        // Save state
        checkpointer.save_state(&thread_id, &state).await.unwrap();

        // Load state
        let loaded = checkpointer.load_state(&thread_id).await.unwrap();
        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();

        assert_eq!(loaded_state.todos.len(), 1);
        assert_eq!(loaded_state.todos[0].content, "Test todo");
        assert_eq!(loaded_state.files.get("test.txt").unwrap(), "content");
        assert_eq!(
            loaded_state.scratchpad.get("key").unwrap(),
            &serde_json::json!("value")
        );
    }

    #[tokio::test]
    async fn in_memory_checkpointer_nonexistent_thread() {
        let checkpointer = InMemoryCheckpointer::new();
        let result = checkpointer
            .load_state(&"nonexistent".to_string())
            .await
            .unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    async fn in_memory_checkpointer_delete_thread() {
        let checkpointer = InMemoryCheckpointer::new();
        let thread_id = "test-thread".to_string();
        let state = sample_state();

        // Save and verify
        checkpointer.save_state(&thread_id, &state).await.unwrap();
        assert!(checkpointer.load_state(&thread_id).await.unwrap().is_some());

        // Delete and verify
        checkpointer.delete_thread(&thread_id).await.unwrap();
        assert!(checkpointer.load_state(&thread_id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn in_memory_checkpointer_list_threads() {
        let checkpointer = InMemoryCheckpointer::new();
        let state = sample_state();

        // Save multiple threads
        checkpointer
            .save_state(&"thread1".to_string(), &state)
            .await
            .unwrap();
        checkpointer
            .save_state(&"thread2".to_string(), &state)
            .await
            .unwrap();

        let threads = checkpointer.list_threads().await.unwrap();
        assert_eq!(threads.len(), 2);
        assert!(threads.contains(&"thread1".to_string()));
        assert!(threads.contains(&"thread2".to_string()));
    }
}
