//! # agents-mcp
//!
//! Native Model Context Protocol (MCP) client implementation for the Rust Deep Agents SDK.
//!
//! This crate provides a minimal, focused MCP client built from scratch without external
//! MCP dependencies. It allows Deep Agents to consume tools from MCP servers.
//!
//! ## Features
//!
//! - **JSON-RPC 2.0**: Full protocol implementation
//! - **Stdio Transport**: Spawn MCP servers as subprocesses
//! - **Tool Adapter**: Seamless conversion of MCP tools to SDK tools
//! - **Zero External MCP Deps**: Only uses serde, tokio, and workspace dependencies
//!
//! ## Example
//!
//! ```rust,ignore
//! use agents_mcp::{McpClient, StdioTransport};
//!
//! // Spawn an MCP server
//! let transport = StdioTransport::spawn("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]).await?;
//!
//! // Connect and initialize
//! let client = McpClient::connect(transport).await?;
//!
//! // List available tools
//! for tool in client.tools() {
//!     println!("Tool: {} - {}", tool.name, tool.description.as_deref().unwrap_or(""));
//! }
//!
//! // Call a tool
//! let result = client.call_tool("read_file", serde_json::json!({"path": "/tmp/test.txt"})).await?;
//! ```

pub mod protocol;
pub mod transport;

mod client;
mod tool_adapter;

// Re-exports
pub use client::{McpClient, McpClientConfig};
pub use protocol::{
    error::McpError,
    types::{McpContent, McpTool, McpToolResult},
};
pub use tool_adapter::{create_mcp_tools, McpToolAdapter};

#[cfg(feature = "stdio")]
pub use transport::stdio::StdioTransport;
