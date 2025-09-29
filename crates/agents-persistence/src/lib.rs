//! Database-backed persistence implementations for agent checkpointing.
//!
//! This crate provides production-ready checkpointer implementations for various
//! storage backends, allowing users to choose the persistence layer that best
//! fits their infrastructure.
//!
//! ## Available Backends
//!
//! - **Redis**: High-performance in-memory data store with optional persistence
//! - **PostgreSQL**: Robust relational database with ACID guarantees
//! - **DynamoDB**: AWS-managed NoSQL database (available in `agents-aws` crate)
//!
//! ## Feature Flags
//!
//! - `redis`: Enable Redis checkpointer
//! - `postgres`: Enable PostgreSQL checkpointer
//! - `all`: Enable all backends
//!
//! ## Examples
//!
//! ### Redis Checkpointer
//!
//! ```rust,no_run
//! use agents_persistence::RedisCheckpointer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let checkpointer = RedisCheckpointer::new("redis://127.0.0.1:6379").await?;
//!     // Use with ConfigurableAgentBuilder
//!     Ok(())
//! }
//! ```
//!
//! ### PostgreSQL Checkpointer
//!
//! ```rust,no_run
//! use agents_persistence::PostgresCheckpointer;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     let checkpointer = PostgresCheckpointer::new(
//!         "postgresql://user:pass@localhost/agents"
//!     ).await?;
//!     // Use with ConfigurableAgentBuilder
//!     Ok(())
//! }
//! ```

#[cfg(feature = "redis")]
pub mod redis_checkpointer;

#[cfg(feature = "postgres")]
pub mod postgres_checkpointer;

#[cfg(feature = "redis")]
pub use redis_checkpointer::RedisCheckpointer;

#[cfg(feature = "postgres")]
pub use postgres_checkpointer::PostgresCheckpointer;

// Re-export core types for convenience
pub use agents_core::persistence::{Checkpointer, ThreadId};
pub use agents_core::state::AgentStateSnapshot;
