//! Redis-backed checkpointer implementation using connection pooling.
//!
//! This checkpointer stores agent state in Redis with automatic serialization
//! and deserialization. It's ideal for:
//! - High-performance applications requiring fast state access
//! - Distributed systems where multiple agent instances share state
//! - Applications already using Redis for caching or session management
//!
//! ## Features
//!
//! - Automatic JSON serialization/deserialization
//! - Connection pooling for efficient resource usage
//! - TTL support for automatic state expiration
//! - Namespace support for multi-tenant applications

use agents_core::persistence::{Checkpointer, ThreadId};
use agents_core::state::AgentStateSnapshot;
use anyhow::Context;
use async_trait::async_trait;
use redis::{aio::ConnectionManager, AsyncCommands};
use std::time::Duration;

/// Redis-backed checkpointer with connection pooling and TTL support.
///
/// # Examples
///
/// ```rust,no_run
/// use agents_persistence::RedisCheckpointer;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Basic usage
///     let checkpointer = RedisCheckpointer::new("redis://127.0.0.1:6379").await?;
///
///     // With namespace and TTL
///     let checkpointer = RedisCheckpointer::builder()
///         .url("redis://127.0.0.1:6379")
///         .namespace("myapp")
///         .ttl(Duration::from_secs(86400)) // 24 hours
///         .build()
///         .await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct RedisCheckpointer {
    connection: ConnectionManager,
    namespace: String,
    ttl: Option<Duration>,
}

impl RedisCheckpointer {
    /// Create a new Redis checkpointer with the default namespace.
    ///
    /// # Arguments
    ///
    /// * `url` - Redis connection URL (e.g., "redis://127.0.0.1:6379")
    pub async fn new(url: &str) -> anyhow::Result<Self> {
        Self::builder().url(url).build().await
    }

    /// Create a builder for configuring the Redis checkpointer.
    pub fn builder() -> RedisCheckpointerBuilder {
        RedisCheckpointerBuilder::default()
    }

    /// Generate the full Redis key for a thread.
    fn key_for_thread(&self, thread_id: &ThreadId) -> String {
        format!("{}:thread:{}", self.namespace, thread_id)
    }

    /// Generate the Redis key for the thread index.
    fn threads_index_key(&self) -> String {
        format!("{}:threads", self.namespace)
    }
}

#[async_trait]
impl Checkpointer for RedisCheckpointer {
    async fn save_state(
        &self,
        thread_id: &ThreadId,
        state: &AgentStateSnapshot,
    ) -> anyhow::Result<()> {
        let key = self.key_for_thread(thread_id);
        let index_key = self.threads_index_key();

        let json =
            serde_json::to_string(state).context("Failed to serialize agent state to JSON")?;

        let mut conn = self.connection.clone();

        // Save the state
        if let Some(ttl) = self.ttl {
            conn.set_ex::<_, _, ()>(&key, json, ttl.as_secs())
                .await
                .context("Failed to save state to Redis with TTL")?;
        } else {
            conn.set::<_, _, ()>(&key, json)
                .await
                .context("Failed to save state to Redis")?;
        }

        // Add to thread index
        conn.sadd::<_, _, ()>(&index_key, thread_id)
            .await
            .context("Failed to update thread index")?;

        tracing::debug!(
            thread_id = %thread_id,
            namespace = %self.namespace,
            "Saved agent state to Redis"
        );

        Ok(())
    }

    async fn load_state(&self, thread_id: &ThreadId) -> anyhow::Result<Option<AgentStateSnapshot>> {
        let key = self.key_for_thread(thread_id);
        let mut conn = self.connection.clone();

        let json: Option<String> = conn
            .get(&key)
            .await
            .context("Failed to load state from Redis")?;

        match json {
            Some(data) => {
                let state: AgentStateSnapshot = serde_json::from_str(&data)
                    .context("Failed to deserialize agent state from JSON")?;

                tracing::debug!(
                    thread_id = %thread_id,
                    namespace = %self.namespace,
                    "Loaded agent state from Redis"
                );

                Ok(Some(state))
            }
            None => {
                tracing::debug!(
                    thread_id = %thread_id,
                    namespace = %self.namespace,
                    "No saved state found in Redis"
                );
                Ok(None)
            }
        }
    }

    async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        let key = self.key_for_thread(thread_id);
        let index_key = self.threads_index_key();
        let mut conn = self.connection.clone();

        // Delete the state
        conn.del::<_, ()>(&key)
            .await
            .context("Failed to delete state from Redis")?;

        // Remove from thread index
        conn.srem::<_, _, ()>(&index_key, thread_id)
            .await
            .context("Failed to update thread index")?;

        tracing::debug!(
            thread_id = %thread_id,
            namespace = %self.namespace,
            "Deleted thread from Redis"
        );

        Ok(())
    }

    async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>> {
        let index_key = self.threads_index_key();
        let mut conn = self.connection.clone();

        let threads: Vec<String> = conn
            .smembers(&index_key)
            .await
            .context("Failed to list threads from Redis")?;

        Ok(threads)
    }
}

/// Builder for configuring a Redis checkpointer.
#[derive(Default)]
pub struct RedisCheckpointerBuilder {
    url: Option<String>,
    namespace: Option<String>,
    ttl: Option<Duration>,
}

impl RedisCheckpointerBuilder {
    /// Set the Redis connection URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the namespace for Redis keys (default: "agents").
    ///
    /// This is useful for multi-tenant applications or when multiple
    /// agent systems share the same Redis instance.
    pub fn namespace(mut self, namespace: impl Into<String>) -> Self {
        self.namespace = Some(namespace.into());
        self
    }

    /// Set the TTL (time-to-live) for stored states.
    ///
    /// After this duration, Redis will automatically delete the state.
    /// Useful for implementing automatic cleanup policies.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Build the Redis checkpointer.
    pub async fn build(self) -> anyhow::Result<RedisCheckpointer> {
        let url = self
            .url
            .ok_or_else(|| anyhow::anyhow!("Redis URL is required"))?;

        let client = redis::Client::open(url.as_str()).context("Failed to create Redis client")?;

        let connection = ConnectionManager::new(client)
            .await
            .context("Failed to establish Redis connection")?;

        Ok(RedisCheckpointer {
            connection,
            namespace: self.namespace.unwrap_or_else(|| "agents".to_string()),
            ttl: self.ttl,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agents_core::state::TodoItem;

    fn sample_state() -> AgentStateSnapshot {
        let mut state = AgentStateSnapshot::default();
        state.todos.push(TodoItem::pending("Test todo"));
        state
            .files
            .insert("test.txt".to_string(), "content".to_string());
        state
            .scratchpad
            .insert("key".to_string(), serde_json::json!("value"));
        state
    }

    #[tokio::test]
    #[ignore] // Requires Redis instance running
    async fn test_redis_save_and_load() {
        let checkpointer = RedisCheckpointer::new("redis://127.0.0.1:6379")
            .await
            .expect("Failed to connect to Redis");

        let thread_id = "test-thread".to_string();
        let state = sample_state();

        // Save state
        checkpointer
            .save_state(&thread_id, &state)
            .await
            .expect("Failed to save state");

        // Load state
        let loaded = checkpointer
            .load_state(&thread_id)
            .await
            .expect("Failed to load state");

        assert!(loaded.is_some());
        let loaded_state = loaded.unwrap();

        assert_eq!(loaded_state.todos.len(), 1);
        assert_eq!(loaded_state.files.get("test.txt").unwrap(), "content");

        // Cleanup
        checkpointer
            .delete_thread(&thread_id)
            .await
            .expect("Failed to delete thread");
    }

    #[tokio::test]
    #[ignore] // Requires Redis instance running
    async fn test_redis_list_threads() {
        let checkpointer = RedisCheckpointer::builder()
            .url("redis://127.0.0.1:6379")
            .namespace("test-namespace")
            .build()
            .await
            .expect("Failed to connect to Redis");

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

        // List threads
        let threads = checkpointer.list_threads().await.unwrap();
        assert!(threads.contains(&"thread1".to_string()));
        assert!(threads.contains(&"thread2".to_string()));

        // Cleanup
        checkpointer
            .delete_thread(&"thread1".to_string())
            .await
            .unwrap();
        checkpointer
            .delete_thread(&"thread2".to_string())
            .await
            .unwrap();
    }
}
