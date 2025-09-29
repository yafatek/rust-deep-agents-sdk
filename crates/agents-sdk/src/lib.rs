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
//! use agents_sdk::{ConfigurableAgentBuilder, get_default_model, create_tool};
//! use serde_json::Value;
//!
//! #[tokio::main]
//! async fn main() -> anyhow::Result<()> {
//!     // Create a simple tool
//!     let my_tool = create_tool(
//!         "greet",
//!         "Greets a person by name",
//!         |args: Value| async move {
//!             let name = args.get("name")
//!                 .and_then(|v| v.as_str())
//!                 .unwrap_or("World");
//!             Ok(format!("Hello, {}!", name))
//!         }
//!     );
//!
//!     // Build an agent with the default Claude model
//!     let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
//!         .with_model(get_default_model()?)
//!         .with_tool(my_tool)
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
//!
//!     Ok(())
//! }
//! ```
//!
//! ## Features
//!
//! - `toolkit` (default): Includes agents-toolkit with built-in tools
//! - `aws`: Includes AWS integrations (DynamoDB, Secrets Manager, etc.)
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
//! # With AWS integrations
//! agents-sdk = { version = "0.0.1", features = ["aws"] }
//!
//! # Everything included
//! agents-sdk = { version = "0.0.1", features = ["full"] }
//! ```

#![deny(missing_docs)]
#![cfg_attr(docsrs, feature(doc_cfg))]

// Re-export core functionality (always available)
pub use agents_core::{agent, hitl, llm, messaging, persistence, state};
pub use agents_runtime::{
    create_async_deep_agent, create_deep_agent, get_default_model, ConfigurableAgentBuilder,
    DeepAgent,
};

// Re-export toolkit functionality (when toolkit feature is enabled)
#[cfg(feature = "toolkit")]
#[cfg_attr(docsrs, doc(cfg(feature = "toolkit")))]
pub use agents_toolkit::*;

// Re-export AWS functionality (when aws feature is enabled)
#[cfg(feature = "aws")]
#[cfg_attr(docsrs, doc(cfg(feature = "aws")))]
pub use agents_aws::*;

/// Prelude module for common imports
///
/// ```rust
/// use agents_sdk::prelude::*;
/// ```
pub mod prelude {

    // Core types
    pub use agents_core::agent::{AgentHandle, PlannerHandle, ToolHandle, ToolResponse};
    pub use agents_core::messaging::{AgentMessage, MessageContent, MessageRole, ToolInvocation};
    pub use agents_core::persistence::{Checkpointer, ThreadId};
    pub use agents_core::state::AgentStateSnapshot;

    // Runtime essentials
    pub use agents_runtime::{get_default_model, ConfigurableAgentBuilder};

    // Toolkit utilities (when available)
    #[cfg(feature = "toolkit")]
    pub use agents_toolkit::{create_sync_tool, create_tool};
}

// Convenience re-exports for the most commonly used items already handled above

#[cfg(feature = "toolkit")]
pub use agents_toolkit::{create_sync_tool, create_tool};
