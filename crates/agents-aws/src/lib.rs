//! AWS integration helpers: wiring for Secrets Manager, DynamoDB, and CloudWatch.
//! Concrete implementations will live behind feature flags, so the core remains
//! lightweight when running outside AWS.
//!
//! ## Features
//!
//! - `dynamodb`: Enable DynamoDB checkpointer for state persistence
//! - `secrets`: Enable AWS Secrets Manager integration
//! - `aws-sdk`: Enable all AWS integrations
//!
//! ## Examples
//!
//! ### DynamoDB Checkpointer
//!
//! ```rust,no_run
//! # #[cfg(feature = "dynamodb")]
//! # {
//! use agents_aws::{DynamoDbCheckpointer, Checkpointer};
//! use agents_core::state::AgentStateSnapshot;
//!
//! # async fn example() -> anyhow::Result<()> {
//! // Create a DynamoDB checkpointer
//! let checkpointer = DynamoDbCheckpointer::new("agent-checkpoints").await?;
//!
//! // Save agent state
//! let state = AgentStateSnapshot::default();
//! checkpointer.save_state(&"thread-id".to_string(), &state).await?;
//!
//! // Load agent state
//! let loaded = checkpointer.load_state(&"thread-id".to_string()).await?;
//! # Ok(())
//! # }
//! # }
//! ```

#[cfg(feature = "dynamodb")]
pub mod dynamodb_checkpointer;

#[cfg(feature = "dynamodb")]
pub use dynamodb_checkpointer::{DynamoDbCheckpointer, DynamoDbCheckpointerBuilder};

// Re-export core types for convenience
pub use agents_core::persistence::{Checkpointer, ThreadId};

/// Placeholder trait for loading configuration secrets.
pub trait SecretsProvider {
    fn fetch(&self, key: &str) -> anyhow::Result<String>;
}

/// Stub Secrets Manager provider; real implementation will sit behind the `secrets` feature.
pub struct UnimplementedSecretsProvider;

impl SecretsProvider for UnimplementedSecretsProvider {
    fn fetch(&self, key: &str) -> anyhow::Result<String> {
        Err(anyhow::anyhow!("Secrets provider not implemented: {key}"))
    }
}
