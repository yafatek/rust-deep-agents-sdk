//! DynamoDB-backed checkpointer implementation for AWS deployments.
//!
//! This checkpointer stores agent state in Amazon DynamoDB, providing:
//! - Fully managed, serverless persistence
//! - Automatic scaling and high availability
//! - Global table support for multi-region deployments
//! - Pay-per-request pricing with on-demand mode
//!
//! ## Table Schema
//!
//! The checkpointer expects a DynamoDB table with the following schema:
//!
//! - **Primary Key**: `thread_id` (String)
//! - **Attributes**:
//!   - `state` (Map/JSON) - The serialized agent state
//!   - `updated_at` (String) - ISO 8601 timestamp
//!   - `ttl` (Number, optional) - Unix epoch for automatic expiration
//!
//! ## Setup
//!
//! Create the table using AWS CLI:
//!
//! ```bash
//! aws dynamodb create-table \
//!   --table-name agent-checkpoints \
//!   --attribute-definitions AttributeName=thread_id,AttributeType=S \
//!   --key-schema AttributeName=thread_id,KeyType=HASH \
//!   --billing-mode PAY_PER_REQUEST
//! ```
//!
//! Or use Terraform (see `deploy/modules/dynamodb/`).

use agents_core::persistence::{Checkpointer, ThreadId};
use agents_core::state::AgentStateSnapshot;
use anyhow::Context;
use async_trait::async_trait;
use aws_sdk_dynamodb::{
    types::AttributeValue,
    Client,
};
use std::collections::HashMap;
use std::time::Duration;

/// DynamoDB-backed checkpointer for serverless AWS deployments.
///
/// # Examples
///
/// ```rust,no_run
/// use agents_aws::DynamoDbCheckpointer;
/// use std::time::Duration;
///
/// #[tokio::main]
/// async fn main() -> anyhow::Result<()> {
///     // Using default AWS configuration
///     let checkpointer = DynamoDbCheckpointer::new("agent-checkpoints").await?;
///
///     // With custom configuration and TTL
///     let checkpointer = DynamoDbCheckpointer::builder()
///         .table_name("my-agents")
///         .ttl(Duration::from_secs(86400 * 7)) // 7 days
///         .build()
///         .await?;
///
///     Ok(())
/// }
/// ```
#[derive(Clone)]
pub struct DynamoDbCheckpointer {
    client: Client,
    table_name: String,
    ttl_seconds: Option<i64>,
}

impl DynamoDbCheckpointer {
    /// Create a new DynamoDB checkpointer with default AWS configuration.
    ///
    /// This will use the default AWS credential chain (environment variables,
    /// IAM roles, AWS config files, etc.).
    ///
    /// # Arguments
    ///
    /// * `table_name` - The name of the DynamoDB table
    pub async fn new(table_name: impl Into<String>) -> anyhow::Result<Self> {
        Self::builder().table_name(table_name).build().await
    }

    /// Create a builder for configuring the DynamoDB checkpointer.
    pub fn builder() -> DynamoDbCheckpointerBuilder {
        DynamoDbCheckpointerBuilder::default()
    }

    /// Calculate TTL timestamp for the current time.
    fn calculate_ttl(&self) -> Option<i64> {
        self.ttl_seconds.map(|ttl| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64
                + ttl
        })
    }
}

#[async_trait]
impl Checkpointer for DynamoDbCheckpointer {
    async fn save_state(
        &self,
        thread_id: &ThreadId,
        state: &AgentStateSnapshot,
    ) -> anyhow::Result<()> {
        let state_json = serde_json::to_string(state)
            .context("Failed to serialize agent state to JSON")?;

        let mut item = HashMap::new();
        item.insert(
            "thread_id".to_string(),
            AttributeValue::S(thread_id.clone()),
        );
        item.insert("state".to_string(), AttributeValue::S(state_json));
        item.insert(
            "updated_at".to_string(),
            AttributeValue::S(chrono::Utc::now().to_rfc3339()),
        );

        // Add TTL if configured
        if let Some(ttl) = self.calculate_ttl() {
            item.insert("ttl".to_string(), AttributeValue::N(ttl.to_string()));
        }

        self.client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .context("Failed to save state to DynamoDB")?;

        tracing::debug!(
            thread_id = %thread_id,
            table = %self.table_name,
            "Saved agent state to DynamoDB"
        );

        Ok(())
    }

    async fn load_state(
        &self,
        thread_id: &ThreadId,
    ) -> anyhow::Result<Option<AgentStateSnapshot>> {
        let mut key = HashMap::new();
        key.insert(
            "thread_id".to_string(),
            AttributeValue::S(thread_id.clone()),
        );

        let result = self
            .client
            .get_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await
            .context("Failed to load state from DynamoDB")?;

        match result.item {
            Some(item) => {
                let state_value = item
                    .get("state")
                    .and_then(|v| v.as_s().ok())
                    .ok_or_else(|| anyhow::anyhow!("State attribute not found or invalid"))?;

                let state: AgentStateSnapshot = serde_json::from_str(state_value)
                    .context("Failed to deserialize agent state from JSON")?;

                tracing::debug!(
                    thread_id = %thread_id,
                    table = %self.table_name,
                    "Loaded agent state from DynamoDB"
                );

                Ok(Some(state))
            }
            None => {
                tracing::debug!(
                    thread_id = %thread_id,
                    table = %self.table_name,
                    "No saved state found in DynamoDB"
                );
                Ok(None)
            }
        }
    }

    async fn delete_thread(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        let mut key = HashMap::new();
        key.insert(
            "thread_id".to_string(),
            AttributeValue::S(thread_id.clone()),
        );

        self.client
            .delete_item()
            .table_name(&self.table_name)
            .set_key(Some(key))
            .send()
            .await
            .context("Failed to delete thread from DynamoDB")?;

        tracing::debug!(
            thread_id = %thread_id,
            table = %self.table_name,
            "Deleted thread from DynamoDB"
        );

        Ok(())
    }

    async fn list_threads(&self) -> anyhow::Result<Vec<ThreadId>> {
        let mut threads = Vec::new();
        let mut last_evaluated_key: Option<HashMap<String, AttributeValue>> = None;

        loop {
            let mut scan = self
                .client
                .scan()
                .table_name(&self.table_name)
                .projection_expression("thread_id");

            if let Some(key) = last_evaluated_key {
                scan = scan.set_exclusive_start_key(Some(key));
            }

            let result = scan
                .send()
                .await
                .context("Failed to list threads from DynamoDB")?;

            if let Some(items) = result.items {
                for item in items {
                    if let Some(thread_id) = item
                        .get("thread_id")
                        .and_then(|v| v.as_s().ok())
                        .map(|s| s.to_string())
                    {
                        threads.push(thread_id);
                    }
                }
            }

            last_evaluated_key = result.last_evaluated_key;

            if last_evaluated_key.is_none() {
                break;
            }
        }

        Ok(threads)
    }
}

/// Builder for configuring a DynamoDB checkpointer.
#[derive(Default)]
pub struct DynamoDbCheckpointerBuilder {
    table_name: Option<String>,
    ttl: Option<Duration>,
    client: Option<Client>,
}

impl DynamoDbCheckpointerBuilder {
    /// Set the DynamoDB table name.
    pub fn table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = Some(table_name.into());
        self
    }

    /// Set the TTL (time-to-live) for stored states.
    ///
    /// DynamoDB will automatically delete items after this duration.
    /// Note: You must enable TTL on the `ttl` attribute in your table.
    pub fn ttl(mut self, ttl: Duration) -> Self {
        self.ttl = Some(ttl);
        self
    }

    /// Use a custom DynamoDB client.
    ///
    /// This is useful for testing with LocalStack or using custom endpoints.
    pub fn client(mut self, client: Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the DynamoDB checkpointer.
    pub async fn build(self) -> anyhow::Result<DynamoDbCheckpointer> {
        let table_name = self
            .table_name
            .ok_or_else(|| anyhow::anyhow!("Table name is required"))?;

        let client = match self.client {
            Some(client) => client,
            None => {
                let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
                Client::new(&config)
            }
        };

        Ok(DynamoDbCheckpointer {
            client,
            table_name,
            ttl_seconds: self.ttl.map(|d| d.as_secs() as i64),
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
    #[ignore] // Requires DynamoDB or LocalStack
    async fn test_dynamodb_save_and_load() {
        let checkpointer = DynamoDbCheckpointer::new("agent-checkpoints-test")
            .await
            .expect("Failed to create DynamoDB client");

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
}

