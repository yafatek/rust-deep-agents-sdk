//! MCP Error Types
//!
//! This module defines error types for MCP operations.

use crate::protocol::messages::JsonRpcError;
use thiserror::Error;

/// MCP Client Error
///
/// Represents all possible errors that can occur during MCP operations.
#[derive(Debug, Error)]
pub enum McpError {
    /// JSON-RPC error returned by the server
    #[error("MCP server error: {0}")]
    ServerError(#[from] JsonRpcError),

    /// Transport-level error (I/O, connection, etc.)
    #[error("Transport error: {0}")]
    Transport(String),

    /// JSON serialization/deserialization error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Protocol error (unexpected message format, version mismatch, etc.)
    #[error("Protocol error: {0}")]
    Protocol(String),

    /// Timeout waiting for response
    #[error("Request timed out after {0:?}")]
    Timeout(std::time::Duration),

    /// Client not initialized
    #[error("Client not initialized - call initialize() first")]
    NotInitialized,

    /// Tool not found
    #[error("Tool not found: {0}")]
    ToolNotFound(String),

    /// Process spawn error
    #[error("Failed to spawn process: {0}")]
    ProcessSpawn(String),

    /// Process exited unexpectedly
    #[error("MCP server process exited unexpectedly")]
    ProcessExited,

    /// Invalid response ID
    #[error("Response ID mismatch: expected {expected}, got {actual}")]
    ResponseIdMismatch { expected: String, actual: String },

    /// Generic I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Generic error wrapper
    #[error("{0}")]
    Other(String),
}

impl McpError {
    /// Create a transport error
    pub fn transport(msg: impl Into<String>) -> Self {
        McpError::Transport(msg.into())
    }

    /// Create a protocol error
    pub fn protocol(msg: impl Into<String>) -> Self {
        McpError::Protocol(msg.into())
    }

    /// Create an "other" error
    pub fn other(msg: impl Into<String>) -> Self {
        McpError::Other(msg.into())
    }

    /// Check if this is a timeout error
    pub fn is_timeout(&self) -> bool {
        matches!(self, McpError::Timeout(_))
    }

    /// Check if this is a server error
    pub fn is_server_error(&self) -> bool {
        matches!(self, McpError::ServerError(_))
    }

    /// Check if the process has exited
    pub fn is_process_exited(&self) -> bool {
        matches!(self, McpError::ProcessExited)
    }
}

/// Result type alias for MCP operations
pub type McpResult<T> = Result<T, McpError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = McpError::ToolNotFound("read_file".to_string());
        assert_eq!(err.to_string(), "Tool not found: read_file");
    }

    #[test]
    fn test_error_helpers() {
        let err = McpError::transport("connection refused");
        assert!(matches!(err, McpError::Transport(_)));

        let err = McpError::protocol("invalid version");
        assert!(matches!(err, McpError::Protocol(_)));
    }

    #[test]
    fn test_error_checks() {
        assert!(McpError::Timeout(std::time::Duration::from_secs(5)).is_timeout());
        assert!(!McpError::ProcessExited.is_timeout());
        assert!(McpError::ProcessExited.is_process_exited());
    }
}
