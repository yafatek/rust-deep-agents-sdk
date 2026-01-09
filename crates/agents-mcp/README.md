# agents-mcp

Native Model Context Protocol (MCP) client for the Rust Deep Agents SDK.

## Overview

This crate provides a minimal, focused MCP client implementation built from scratch without external MCP dependencies. It enables Deep Agents to consume tools from any MCP server.

## Features

- **JSON-RPC 2.0** — Full protocol implementation
- **Stdio Transport** — Spawn MCP servers as subprocesses
- **Tool Adapter** — Seamless conversion of MCP tools to SDK tools
- **Namespace Support** — Avoid tool name collisions with multiple servers
- **Zero External MCP Deps** — Only uses serde, tokio, and workspace dependencies

## Installation

This crate is typically used through the `agents-sdk` with the `mcp` feature:

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["mcp"] }
```

Or directly:

```toml
[dependencies]
agents-mcp = "0.0.29"
```

## Quick Start

```rust
use agents_mcp::{McpClient, StdioTransport, create_mcp_tools};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Spawn an MCP filesystem server
    let transport = StdioTransport::spawn(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    ).await?;

    // Connect and initialize
    let client = Arc::new(McpClient::connect(transport).await?);

    // List available tools
    for tool in client.tools() {
        println!("Tool: {} - {}", 
            tool.name, 
            tool.description.as_deref().unwrap_or("")
        );
    }

    // Call a tool directly
    let result = client.call_tool(
        "read_file",
        serde_json::json!({"path": "/tmp/test.txt"})
    ).await?;

    // Or convert to SDK tools for use with agents
    let sdk_tools = create_mcp_tools(client, Some("fs"));
    // Tools will be named "fs.read_file", "fs.write_file", etc.

    Ok(())
}
```

## API Overview

### McpClient

The main client for communicating with MCP servers:

```rust
// Connect with default config
let client = McpClient::connect(transport).await?;

// Connect with custom config
let config = McpClientConfig {
    request_timeout: Duration::from_secs(60),
    auto_list_tools: true,
    client_name: Some("my-app".to_string()),
    ..Default::default()
};
let client = McpClient::connect_with_config(transport, config).await?;

// Get tools
let tools = client.tools();

// Call a tool
let result = client.call_tool("tool_name", args).await?;

// Check connection
let connected = client.is_connected().await;

// Close gracefully
client.close().await?;
```

### StdioTransport

Spawn MCP servers as subprocesses:

```rust
// Simple spawn
let transport = StdioTransport::spawn("npx", &["-y", "mcp-server"]).await?;

// With full config
let config = StdioConfig::new("python")
    .args(["server.py", "--mode", "production"])
    .env("DEBUG", "true")
    .working_dir("/app");
let transport = StdioTransport::spawn_with_config(config).await?;
```

### McpToolAdapter

Wraps MCP tools to implement the SDK's `Tool` trait:

```rust
// Create adapter for a single tool
let adapter = McpToolAdapter::new(client.clone(), tool)
    .with_namespace("fs");
let toolbox = adapter.into_toolbox();

// Or create adapters for all tools at once
let all_tools = create_mcp_tools(client, Some("fs"));
```

## Error Handling

```rust
use agents_mcp::McpError;

match result {
    Err(McpError::Timeout(d)) => println!("Timed out after {:?}", d),
    Err(McpError::ProcessExited) => println!("Server died"),
    Err(McpError::ServerError(e)) => println!("Server error: {}", e),
    Err(McpError::Transport(msg)) => println!("Transport error: {}", msg),
    Err(e) => println!("Error: {}", e),
    Ok(result) => { /* success */ }
}
```

## Protocol Support

Currently implemented:
- ✅ `initialize` / `initialized` handshake
- ✅ `tools/list` — List available tools
- ✅ `tools/call` — Execute tools

Not yet implemented:
- ❌ `resources/list`, `resources/read` — Resource management
- ❌ `prompts/list`, `prompts/get` — Prompt templates
- ❌ HTTP/SSE transport

## Related

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [MCP Servers Directory](https://github.com/modelcontextprotocol/servers)
- [agents-sdk Documentation](https://docs.rs/agents-sdk)

## License

MIT OR Apache-2.0
