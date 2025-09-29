//! Deep Agent implementation modules
//! 
//! This module contains the core Deep Agent implementation split into logical components:
//! - `api`: Public API functions that mirror the Python SDK exactly
//! - `config`: Configuration structs and builders
//! - `runtime`: Core DeepAgent runtime implementation
//! - `builder`: Fluent builder pattern for agent construction

pub mod api;
pub mod config;
pub mod runtime;
pub mod builder;

// Re-export the main public API
pub use api::{create_deep_agent, create_async_deep_agent, get_default_model};
pub use builder::ConfigurableAgentBuilder;
pub use config::{DeepAgentConfig, SubAgentConfig, SummarizationConfig, CreateDeepAgentParams};
pub use runtime::DeepAgent;
