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
//! use agents_aws::DynamoDbCheckpointer;
//! use agents_sdk::ConfigurableAgentBuilder;
//! use std::sync::Arc;
//!
//! # #[tokio::main]
//! # async fn main() -> anyhow::Result<()> {
//! let checkpointer = Arc::new(
//!     DynamoDbCheckpointer::new("agent-checkpoints").await?
//! );
//!
//! let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
//!     .with_checkpointer(checkpointer)
//!     .build()?;
//! # Ok(())
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
