//! PostgreSQL-backed checkpointer implementation with ACID guarantees.
//!
//! This checkpointer stores agent state in a PostgreSQL database, providing:
//! - ACID transaction guarantees
//! - Persistent storage with backup capabilities
//! - SQL querying for analytics and debugging
//! - Multi-region replication support
//!
//! ## Schema
//!
//! The checkpointer automatically creates the following table:
//!
//! ```sql
//! CREATE TABLE IF NOT EXISTS agent_checkpoints (
//!     thread_id TEXT PRIMARY KEY,
//!     state JSONB NOT NULL,
//!     created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
//!     updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
//! );
//! ```

use agents_core::persistence::{Checkpointer, ThreadId};
use agents_core::state::AgentStateSnapshot;
use anyhow::Context;
use async_trait::async_trait;
use sqlx::{postgres::PgPoolOptions, PgPool, Row};

/// PostgreSQL-backed checkpointer with connection pooling.
///
/// # Examples
///
/// ```rust,no_run
/// use agents_persistence::PostgresCheckpointer;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Basic usage
///     let checkpointer = PostgresCheckpointer::new(
///         "postgresql://user:pass@localhost/agents"
///     ).await?;
///
///     // With custom pool configuration
///     let checkpointer = PostgresCheckpointer::builder()
///         .url("postgresql://user:pass@localhost/agents")
///         .table_name("my_checkpoints")
///         .max_connections(20)
///         .build()
///         .await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct PostgresCheckpointer {
    pool: PgPool,
    table_name: String,
}

impl PostgresCheckpointer {
    /// Create a new PostgreSQL checkpointer with default settings.
    ///
    /// This will automatically create the checkpoints table if it doesn't exist.
    ///
    /// # Arguments
    ///
    /// * `database_url` - PostgreSQL connection string
    pub async fn new(database_url: &str) -> anyhow::Result<Self> {
        Self::builder().url(database_url).build().await
    }

    /// Create a builder for configuring the PostgreSQL checkpointer.
    pub fn builder() -> PostgresCheckpointerBuilder {
        PostgresCheckpointerBuilder::default()
    }

    /// Ensure the checkpoints table exists.
    async fn ensure_table(&self) -> anyhow::Result<()> {
        // Create table
        let create_table_sql = format!(
            r#"
            CREATE TABLE IF NOT EXISTS {} (
                thread_id TEXT PRIMARY KEY,
                state JSONB NOT NULL,
                created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
            )
            "#,
            self.table_name
        );

        sqlx::query(&create_table_sql)
            .execute(&self.pool)
            .await
            .context("Failed to create checkpoints table")?;

        // Create index (separate query)
        let create_index_sql = format!(
            r#"
            CREATE INDEX IF NOT EXISTS idx_{}_updated_at 
            ON {} (updated_at DESC)
            "#,
            self.table_name, self.table_name
        );

        sqlx::query(&create_index_sql)
            .execute(&self.pool)
            .await
            .context("Failed to create index")?;

        Ok(())
    }
}

#[async_trait]
impl Checkpointer for PostgresCheckpointer {
    async fn save_state(
        &self,
        thread_id: &ThreadId,
        state: &AgentStateSnapshot,
    ) -> anyhow::Result<()> {
        let json =
            serde_json::to_value(state).context("Failed to serialize agent state to JSON")?;

        let query = format!(
            r#"
            INSERT INTO {} (thread_id, state, created_at, updated_at)
            VALUES ($1, $2, NOW(), NOW())
            ON CONFLICT (thread_id) 
            DO UPDATE SET state = $2, updated_at = NOW()
            "#,
            self.table_name
        );

        sqlx::query(&query)
            .bind(thread_id)
            .bind(&json)
            .execute(&self.pool)
            .await
            .context("Failed to save state to PostgreSQL")?;

        tracing::debug!(
            thread_id = %thread_id,
            table = %self.table_name,
            "Saved agent state to PostgreSQL"
        );

        Ok(())
    }

    async fn load_state(&self, thread_id: &ThreadId) -> anyhow::Result<Option<AgentStateSnapshot>> {
        let query = format!(
            r#"
            SELECT state FROM {} WHERE thread_id = $1
            "#,
            self.table_name
        );

        let row: Option<(serde_json::Value,)> = sqlx::query_as(&query)
            .bind(thread_id)
            .fetch_optional(&self.pool)
            .await
            .context("Failed to load state from PostgreSQL")?;

        match row {
            Some((json,)) => {
                let state: AgentStateSnapshot = serde_json::from_value(json)
                    .context("Failed to deserialize agent state from JSON")?;

                tracing::debug!(
                    thread_id = %thread_id,
                    table = %self.table_name,
                    "Loaded agent state from PostgreSQL"
                );

                Ok(Some(state))
            }
            None => {
                tracing::debug!(
                    thread_id = %thread_id,
                    table = %self.table_name,
                    "No saved state found in PostgreSQL"
                );
                Ok(None)
            }
        }
    }

    async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        let query = format!(
            r#"
            DELETE FROM {} WHERE thread_id = $1
            "#,
            self.table_name
        );

        sqlx::query(&query)
            .bind(thread_id)
            .execute(&self.pool)
            .await
            .context("Failed to delete thread from PostgreSQL")?;

        tracing::debug!(
            thread_id = %thread_id,
            table = %self.table_name,
            "Deleted thread from PostgreSQL"
        );

        Ok(())
    }

    async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>> {
        let query = format!(
            r#"
            SELECT thread_id FROM {} ORDER BY updated_at DESC
            "#,
            self.table_name
        );

        let rows = sqlx::query(&query)
            .fetch_all(&self.pool)
            .await
            .context("Failed to list threads from PostgreSQL")?;

        let threads = rows
            .into_iter()
            .map(|row| row.get::<String, _>("thread_id"))
            .collect();

        Ok(threads)
    }
}

/// Builder for configuring a PostgreSQL checkpointer.
#[derive(Default)]
pub struct PostgresCheckpointerBuilder {
    url: Option<String>,
    table_name: Option<String>,
    max_connections: Option<u32>,
    min_connections: Option<u32>,
}

impl PostgresCheckpointerBuilder {
    /// Set the PostgreSQL connection URL.
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    /// Set the table name for storing checkpoints (default: "agent_checkpoints").
    pub fn table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }

    /// Set the maximum number of connections in the pool (default: 10).
    pub fn max_connections(mut self, max: u32) -> Self {
        self.max_connections = Some(max);
        self
    }

    /// Set the minimum number of connections in the pool (default: 2).
    pub fn min_connections(mut self, min: u32) -> Self {
        self.min_connections = Some(min);
        self
    }

    /// Build the PostgreSQL checkpointer and initialize the table.
    pub async fn build(self) -> anyhow::Result<PostgresCheckpointer> {
        let url = self
            .url
            .ok_or_else(|| anyhow::anyhow!("PostgreSQL URL is required"))?;

        let mut pool_options = PgPoolOptions::new();

        if let Some(max) = self.max_connections {
            pool_options = pool_options.max_connections(max);
        } else {
            pool_options = pool_options.max_connections(10);
        }

        if let Some(min) = self.min_connections {
            pool_options = pool_options.min_connections(min);
        }

        let pool = pool_options
            .connect(&url)
            .await
            .context("Failed to connect to PostgreSQL")?;

        let checkpointer = PostgresCheckpointer {
            pool,
            table_name: self
                .table_name
                .unwrap_or_else(|| "agent_checkpoints".to_string()),
        };

        // Ensure table exists
        checkpointer
            .ensure_table()
            .await
            .context("Failed to initialize database schema")?;

        Ok(checkpointer)
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
    #[ignore] // Requires PostgreSQL instance running
    async fn test_postgres_save_and_load() {
        let checkpointer = PostgresCheckpointer::new("postgresql://localhost/agents_test")
            .await
            .expect("Failed to connect to PostgreSQL");

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
    #[ignore] // Requires PostgreSQL instance running
    async fn test_postgres_list_threads() {
        let checkpointer = PostgresCheckpointer::builder()
            .url("postgresql://localhost/agents_test")
            .table_name("test_checkpoints")
            .build()
            .await
            .expect("Failed to connect to PostgreSQL");

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
