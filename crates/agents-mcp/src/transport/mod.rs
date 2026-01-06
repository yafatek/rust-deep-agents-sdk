//! MCP Transport Layer
//!
//! This module defines the transport abstraction for MCP communication.
//! Transports handle the low-level message sending and receiving.

#[cfg(feature = "stdio")]
pub mod stdio;

use crate::protocol::McpError;
use async_trait::async_trait;

/// Transport trait for MCP communication
///
/// Implementations handle the actual message sending and receiving
/// over different channels (stdio, HTTP, WebSocket, etc.)
#[async_trait]
pub trait Transport: Send + Sync {
    /// Send a message to the MCP server
    async fn send(&mut self, message: &str) -> Result<(), McpError>;

    /// Receive a message from the MCP server
    async fn receive(&mut self) -> Result<String, McpError>;

    /// Close the transport connection
    async fn close(&mut self) -> Result<(), McpError>;

    /// Check if the transport is still connected
    fn is_connected(&self) -> bool;
}
