//! # Rust Deep Agents SDK
//!
//! High-performance Rust framework for composing reusable "deep" AI agents with custom tools,
//! sub-agents, and prompts.
//!
//! ## Quick Start
//!
//! ```toml
//! [dependencies]
//! agents-sdk = "0.0.1"  # Includes toolkit by default
//! ```
//!
//! ```rust,no_run
//! # #[cfg(feature = "toolkit")]
//! # {
//! use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
//! use agents_core::persistence::InMemoryCheckpointer;
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//!     let config = OpenAiConfig::new(
//!         std::env::var("OPENAI_API_KEY")?,
//!         "gpt-4o-mini"
//!     );
//!
//!     // Create the model
//!     let model = Arc::new(OpenAiChatModel::new(config)?);
//!
//!     // Build an agent
//!     let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
//!         .with_model(model)
//!         .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
//!         .build()?;
//!
//!     // Use the agent
//!     use agents_sdk::state::AgentStateSnapshot;
//!     use std::sync::Arc;
//!
//!     let response = agent.handle_message(
//!         "Please greet Alice using the greet tool",
//!         Arc::new(AgentStateSnapshot::default())
//!     ).await?;
//!     println!("{:?}", response);
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ## Features
//!
//! - `toolkit` (default): Includes agents-toolkit with built-in tools
//! - `aws`: Includes AWS integrations
//! - `redis`: Redis-backed state persistence
//! - `postgres`: PostgreSQL-backed state persistence
//! - `dynamodb`: DynamoDB-backed state persistence (AWS)
//! - `persistence`: Grouped feature for Redis + PostgreSQL
//! - `aws-full`: Grouped feature for AWS + DynamoDB
//! - `full`: Includes all features
//!
//! ## Installation Options
//!
//! ```toml
//! # Default installation with toolkit
//! agents-sdk = "0.0.1"
//!
//! # Core only (minimal installation)
//! agents-sdk = { version = "0.0.1", default-features = false }
//!
//! # With specific persistence backend
//! agents-sdk = { version = "0.0.1", features = ["redis"] }
//! agents-sdk = { version = "0.0.1", features = ["postgres"] }
//! agents-sdk = { version = "0.0.1", features = ["dynamodb"] }
//!
//! # With AWS integrations
//! agents-sdk = { version = "0.0.1", features = ["aws-full"] }
//!
//! # Everything included
//! agents-sdk = { version = "0.0.1", features = ["full"] }
//! ```
//!
//! ## Persistence Examples
//!
//! ### Redis Checkpointer
//!
//! ```rust,no_run
//! # #[cfg(feature = "redis")]
//! # {
//! use agents_sdk::{RedisCheckpointer, ConfigurableAgentBuilder};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let checkpointer = Arc::new(
//!     RedisCheckpointer::new("redis://127.0.0.1:6379").await?
//! );
//!
//! let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
//!     .with_checkpointer(checkpointer)
//!     .build()?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ### PostgreSQL Checkpointer
//!
//! ```rust,no_run
//! # #[cfg(feature = "postgres")]
//! # {
//! use agents_sdk::{PostgresCheckpointer, ConfigurableAgentBuilder};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let checkpointer = Arc::new(
//!     PostgresCheckpointer::new("postgresql://user:pass@localhost/agents").await?
//! );
//!
//! let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
//!     .with_checkpointer(checkpointer)
//!     .build()?;
//! # Ok(())
//! # }
//! # }
//! ```
//!
//! ### DynamoDB Checkpointer
//!
//! ```rust,no_run
//! # #[cfg(feature = "dynamodb")]
//! # {
//! use agents_sdk::{DynamoDbCheckpointer, ConfigurableAgentBuilder};
//! use std::sync::Arc;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let checkpointer = Arc::new(
//!     DynamoDbCheckpointer::new("agent-checkpoints").await?
//! );
//!
//! let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
//!     .with_checkpointer(checkpointer)
//!     .build()?;
//! # Ok(())
//! # }
//! # }
//! ```

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export core functionality (always available)
pub use agents_core::agent::{AgentHandle, AgentStream};
pub use agents_core::llm::{ChunkStream, StreamChunk};
pub use agents_core::tools::{
    Tool, ToolBox, ToolContext, ToolParameterSchema, ToolRegistry, ToolResult, ToolSchema,
};
pub use agents_core::{agent, events, hitl, llm, messaging, persistence, security, state, tools};
pub use agents_runtime::{
    create_async_deep_agent,
    create_deep_agent,
    get_default_model,
    // Provider configurations and models
    AnthropicConfig,
    AnthropicMessagesModel,
    ConfigurableAgentBuilder,
    DeepAgent,
    GeminiChatModel,
    GeminiConfig,
    HitlPolicy,
    OpenAiChatModel,
    OpenAiConfig,
    SubAgentConfig,
    SummarizationConfig,
};

// Re-export token tracking functionality
pub use agents_core::events::TokenUsage;
pub use agents_runtime::middleware::token_tracking::{
    TokenCosts, TokenTrackingConfig, TokenTrackingMiddleware, TokenUsageSummary,
};

// Re-export toolkit functionality (when toolkit feature is enabled)
#[cfg(feature = "toolkit")]
#[cfg_attr(docsrs, doc(cfg(feature = "toolkit")))]
pub use agents_toolkit::*;

// Re-export procedural macros from toolkit
#[cfg(feature = "toolkit")]
pub use agents_macros::tool;

// Re-export AWS functionality (when aws feature is enabled)
#[cfg(feature = "aws")]
#[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
pub use agents_aws::*;

// Re-export persistence functionality (when persistence features are enabled)
#[cfg(feature = "redis")]
#[cfg_attr(docsrs, doc(cfg(feature = "redis")))]
pub use agents_persistence::RedisCheckpointer;

#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub use agents_persistence::PostgresCheckpointer;

/// Prelude module for common imports
///
/// ```rust
/// use agents_sdk::prelude::*;
/// ```
pub mod prelude {

    // Core types
    pub use agents_core::agent::{AgentHandle, PlannerHandle};
    pub use agents_core::messaging::{AgentMessage, MessageContent, MessageRole, ToolInvocation};
    pub use agents_core::persistence::{Checkpointer, ThreadId};
    pub use agents_core::state::AgentStateSnapshot;

    // Runtime essentials
    pub use agents_runtime::{get_default_model, ConfigurableAgentBuilder};

    // Toolkit utilities (when available)
    #[cfg(feature = "toolkit")]
    pub use agents_toolkit::{tool, tool_sync, ToolBuilder};
}

// Convenience re-exports for the most commonly used items already handled above

#[cfg(feature = "toolkit")]
pub use agents_toolkit::{tool_sync, ToolBuilder};
