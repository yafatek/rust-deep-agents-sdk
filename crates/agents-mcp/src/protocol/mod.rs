//! MCP Protocol implementation
//!
//! This module contains the core protocol types for MCP communication:
//! - JSON-RPC 2.0 message types
//! - MCP-specific types (tools, resources, content)
//! - Error handling

pub mod error;
pub mod messages;
pub mod types;

pub use error::McpError;
pub use messages::*;
pub use types::*;
