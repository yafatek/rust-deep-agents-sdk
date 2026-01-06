# MCP (Model Context Protocol) Integration Plan

## Overview

This document outlines the plan to add **native MCP support** to the Rust Deep Agents SDK, built **from scratch without external MCP dependencies**.

## Decision: Build From Scratch

### Why Not Use External Crates?

| Concern | Details |
|---------|---------|
| **Maturity** | `rust-mcp-sdk` has ~133 stars, under heavy development |
| **Breaking Changes** | External crates may introduce breaking changes |
| **Dependency Bloat** | We only need client-side functionality |
| **Control** | Full control over implementation and API design |
| **Learning** | Community can learn from our implementation |

### Our Approach

Build a minimal, focused MCP client implementation using only:
- `serde` / `serde_json` (already in our deps)
- `tokio` (already in our deps)
- `tokio::process` for stdio transport
- `reqwest` (optional, for HTTP transport)

## MCP Protocol Overview

MCP is built on **JSON-RPC 2.0** with specific message types:

### Core Message Flow

```
┌──────────────┐                    ┌──────────────┐
│   Agent      │                    │  MCP Server  │
│  (Client)    │                    │              │
└──────┬───────┘                    └──────┬───────┘
       │                                   │
       │──── initialize ─────────────────► │
       │◄─── initialize result ─────────── │
       │                                   │
       │──── initialized ────────────────► │
       │                                   │
       │──── tools/list ─────────────────► │
       │◄─── tools list ─────────────────  │
       │                                   │
       │──── tools/call ─────────────────► │
       │◄─── tool result ────────────────  │
       │                                   │
```

### JSON-RPC Message Format

```rust
// Request
{
    "jsonrpc": "2.0",
    "id": 1,
    "method": "tools/list",
    "params": {}
}

// Response
{
    "jsonrpc": "2.0",
    "id": 1,
    "result": {
        "tools": [
            {
                "name": "read_file",
                "description": "Read contents of a file",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "path": { "type": "string" }
                    },
                    "required": ["path"]
                }
            }
        ]
    }
}
```

## Architecture Design

### New Crate: `agents-mcp`

```
crates/
├── agents-mcp/
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs           # Public API exports
│       ├── protocol/
│       │   ├── mod.rs
│       │   ├── messages.rs  # JSON-RPC message types
│       │   ├── types.rs     # MCP-specific types (Tool, Resource, etc.)
│       │   └── error.rs     # MCP error types
│       ├── transport/
│       │   ├── mod.rs
│       │   ├── stdio.rs     # Stdio transport (subprocess)
│       │   └── http.rs      # HTTP/SSE transport (optional)
│       ├── client.rs        # MCP client implementation
│       └── tool_adapter.rs  # Convert MCP tools to SDK ToolBox
```

### Core Types (What We Implement)

```rust
// ============================================
// crates/agents-mcp/src/protocol/messages.rs
// ============================================

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str, // Always "2.0"
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: u64,
    #[serde(default)]
    pub result: Option<Value>,
    #[serde(default)]
    pub error: Option<JsonRpcError>,
}

/// JSON-RPC 2.0 Error
#[derive(Debug, Clone, Deserialize)]
pub struct JsonRpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

// ============================================
// crates/agents-mcp/src/protocol/types.rs
// ============================================

/// MCP Tool Definition
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct McpTool {
    pub name: String,
    pub description: Option<String>,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value, // JSON Schema
}

/// MCP Tool Call Result
#[derive(Debug, Clone, Deserialize)]
pub struct McpToolResult {
    pub content: Vec<McpContent>,
    #[serde(rename = "isError", default)]
    pub is_error: bool,
}

/// MCP Content (text, image, etc.)
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum McpContent {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "image")]
    Image { data: String, mime_type: String },
    #[serde(rename = "resource")]
    Resource { uri: String, text: Option<String> },
}

/// Initialize request params
#[derive(Debug, Clone, Serialize)]
pub struct InitializeParams {
    #[serde(rename = "protocolVersion")]
    pub protocol_version: String,
    pub capabilities: ClientCapabilities,
    #[serde(rename = "clientInfo")]
    pub client_info: ClientInfo,
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientCapabilities {
    // We only need tool-calling capability
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientInfo {
    pub name: String,
    pub version: String,
}

/// Tools list response
#[derive(Debug, Clone, Deserialize)]
pub struct ToolsListResult {
    pub tools: Vec<McpTool>,
}

/// Tool call params
#[derive(Debug, Clone, Serialize)]
pub struct ToolCallParams {
    pub name: String,
    pub arguments: Value,
}
```

### MCP Client Implementation

```rust
// ============================================
// crates/agents-mcp/src/client.rs
// ============================================

use crate::protocol::*;
use crate::transport::Transport;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct McpClient {
    transport: Arc<Mutex<Box<dyn Transport>>>,
    request_id: AtomicU64,
    server_name: String,
    tools: Vec<McpTool>,
}

impl McpClient {
    /// Connect to an MCP server and perform initialization
    pub async fn connect(transport: impl Transport + 'static) -> Result<Self, McpError> {
        let mut client = Self {
            transport: Arc::new(Mutex::new(Box::new(transport))),
            request_id: AtomicU64::new(1),
            server_name: String::new(),
            tools: Vec::new(),
        };
        
        // Perform MCP handshake
        client.initialize().await?;
        
        // Fetch available tools
        client.tools = client.list_tools().await?;
        
        Ok(client)
    }
    
    async fn initialize(&mut self) -> Result<(), McpError> {
        let params = InitializeParams {
            protocol_version: "2024-11-05".to_string(),
            capabilities: ClientCapabilities {},
            client_info: ClientInfo {
                name: "rust-deep-agents-sdk".to_string(),
                version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        
        let response = self.send_request("initialize", Some(params)).await?;
        
        // Send initialized notification
        self.send_notification("notifications/initialized", None::<()>).await?;
        
        Ok(())
    }
    
    async fn list_tools(&self) -> Result<Vec<McpTool>, McpError> {
        let response: ToolsListResult = self.send_request("tools/list", None::<()>).await?;
        Ok(response.tools)
    }
    
    /// Call a tool on the MCP server
    pub async fn call_tool(&self, name: &str, arguments: Value) -> Result<McpToolResult, McpError> {
        let params = ToolCallParams {
            name: name.to_string(),
            arguments,
        };
        self.send_request("tools/call", Some(params)).await
    }
    
    /// Get list of available tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }
    
    async fn send_request<P: Serialize, R: DeserializeOwned>(
        &self,
        method: &str,
        params: Option<P>,
    ) -> Result<R, McpError> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params: params.map(|p| serde_json::to_value(p).unwrap()),
        };
        
        let mut transport = self.transport.lock().await;
        transport.send(&serde_json::to_string(&request)?).await?;
        
        let response_str = transport.receive().await?;
        let response: JsonRpcResponse = serde_json::from_str(&response_str)?;
        
        if let Some(error) = response.error {
            return Err(McpError::ServerError(error));
        }
        
        serde_json::from_value(response.result.unwrap_or(Value::Null))
            .map_err(McpError::from)
    }
}
```

### Stdio Transport

```rust
// ============================================
// crates/agents-mcp/src/transport/stdio.rs
// ============================================

use async_trait::async_trait;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, Command};

pub struct StdioTransport {
    child: Child,
    stdin: tokio::process::ChildStdin,
    stdout: BufReader<tokio::process::ChildStdout>,
}

impl StdioTransport {
    pub async fn spawn(command: &str, args: &[&str]) -> Result<Self, McpError> {
        let mut child = Command::new(command)
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::inherit())
            .spawn()?;
        
        let stdin = child.stdin.take().expect("stdin not captured");
        let stdout = BufReader::new(child.stdout.take().expect("stdout not captured"));
        
        Ok(Self { child, stdin, stdout })
    }
}

#[async_trait]
impl Transport for StdioTransport {
    async fn send(&mut self, message: &str) -> Result<(), McpError> {
        self.stdin.write_all(message.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }
    
    async fn receive(&mut self) -> Result<String, McpError> {
        let mut line = String::new();
        self.stdout.read_line(&mut line).await?;
        Ok(line)
    }
}
```

### Tool Adapter (MCP → SDK)

```rust
// ============================================
// crates/agents-mcp/src/tool_adapter.rs
// ============================================

use agents_core::tools::{Tool, ToolBox, ToolContext, ToolResult, ToolSchema};
use crate::{McpClient, McpTool, McpContent};
use std::sync::Arc;

/// Wraps an MCP tool to implement our SDK's Tool trait
pub struct McpToolAdapter {
    client: Arc<McpClient>,
    tool: McpTool,
}

impl McpToolAdapter {
    pub fn new(client: Arc<McpClient>, tool: McpTool) -> Self {
        Self { client, tool }
    }
    
    /// Convert MCP tool to SDK ToolBox
    pub fn into_toolbox(self) -> ToolBox {
        ToolBox::new(Arc::new(self))
    }
}

#[async_trait::async_trait]
impl Tool for McpToolAdapter {
    fn schema(&self) -> ToolSchema {
        ToolSchema {
            name: self.tool.name.clone(),
            description: self.tool.description.clone().unwrap_or_default(),
            parameters: self.tool.input_schema.clone(),
        }
    }
    
    async fn execute(
        &self,
        args: serde_json::Value,
        _ctx: ToolContext,
    ) -> anyhow::Result<ToolResult> {
        let result = self.client.call_tool(&self.tool.name, args).await?;
        
        // Convert MCP result to SDK result
        let content = result.content
            .into_iter()
            .filter_map(|c| match c {
                McpContent::Text { text } => Some(text),
                McpContent::Resource { text, .. } => text,
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n");
        
        if result.is_error {
            Ok(ToolResult::error(content))
        } else {
            Ok(ToolResult::success(content))
        }
    }
}
```

### Builder Integration

```rust
// ============================================
// Extension to ConfigurableAgentBuilder
// ============================================

impl ConfigurableAgentBuilder {
    /// Add tools from an MCP server via stdio (subprocess)
    pub async fn with_mcp_stdio(
        mut self,
        command: &str,
        args: &[&str],
    ) -> Result<Self, McpError> {
        let transport = StdioTransport::spawn(command, args).await?;
        let client = Arc::new(McpClient::connect(transport).await?);
        
        // Convert all MCP tools to SDK tools
        for mcp_tool in client.tools().iter().cloned() {
            let adapter = McpToolAdapter::new(client.clone(), mcp_tool);
            self.tools.push(adapter.into_toolbox());
        }
        
        self.mcp_clients.push(client);
        Ok(self)
    }
    
    /// Add tools from an MCP server via HTTP
    #[cfg(feature = "mcp-http")]
    pub async fn with_mcp_http(
        mut self,
        url: &str,
    ) -> Result<Self, McpError> {
        let transport = HttpTransport::connect(url).await?;
        let client = Arc::new(McpClient::connect(transport).await?);
        
        for mcp_tool in client.tools().iter().cloned() {
            let adapter = McpToolAdapter::new(client.clone(), mcp_tool);
            self.tools.push(adapter.into_toolbox());
        }
        
        self.mcp_clients.push(client);
        Ok(self)
    }
}
```

## Implementation Phases

### Phase 1: Core Protocol (MVP)
**Estimated: 3-4 days**

- [ ] Create `agents-mcp` crate structure
- [ ] Implement JSON-RPC message types
- [ ] Implement MCP protocol types (Tool, Content, etc.)
- [ ] Implement basic `McpClient`
- [ ] Implement `StdioTransport`
- [ ] Implement `McpToolAdapter`
- [ ] Add `with_mcp_stdio()` to builder
- [ ] Basic example with filesystem server

**Dependencies**: Only `serde`, `serde_json`, `tokio`, `async-trait` (all already in deps)

### Phase 2: Robustness
**Estimated: 2 days**

- [ ] Error handling and recovery
- [ ] Connection health checks
- [ ] Graceful shutdown (kill subprocess)
- [ ] Timeout handling
- [ ] Logging with `tracing`

### Phase 3: HTTP Transport (Optional)
**Estimated: 2-3 days**

- [ ] HTTP/SSE transport implementation
- [ ] Connection pooling
- [ ] `with_mcp_http()` builder method
- [ ] Feature-gated behind `mcp-http`

### Phase 4: Advanced Features
**Estimated: 2-3 days**

- [ ] MCP Resources support
- [ ] MCP Prompts support
- [ ] Tool namespacing (prefix with server name)
- [ ] Parallel tool calls optimization

## Dependencies

```toml
# crates/agents-mcp/Cargo.toml
[package]
name = "agents-mcp"
version = "0.1.0"
edition = "2021"

[dependencies]
agents-core = { path = "../agents-core" }

# Serialization (already in workspace)
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"

# Async runtime (already in workspace)
tokio = { version = "1.0", features = ["process", "io-util", "sync"] }
async-trait = "0.1"

# Logging (already in workspace)
tracing = "0.1"

# Error handling (already in workspace)
anyhow = "1.0"
thiserror = "1.0"

[features]
default = ["stdio"]
stdio = []
http = ["reqwest"]

[dependencies.reqwest]
version = "0.12"
optional = true
features = ["json"]
```

## Example Usage

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    // Build agent with MCP filesystem server
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant with filesystem access."
    )
    .with_model(model)
    // Spawn filesystem MCP server as subprocess
    .with_mcp_stdio("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
    .await?
    // Can still add native tools
    .with_tool(MyCustomTool::as_tool())
    .build()?;

    // Agent now has access to: read_file, write_file, list_directory, etc.
    let response = agent.handle_message(
        "List the files in /tmp directory",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());
    Ok(())
}
```

## Testing Strategy

1. **Mock MCP Server**: Create a simple in-process MCP server for testing
2. **Unit Tests**: Test JSON-RPC serialization, type conversion
3. **Integration Tests**: Test with real `@modelcontextprotocol/server-filesystem`
4. **Example Programs**: Demonstrate various use cases

## Documentation Updates

- [ ] Add `book/src/features/mcp.md`
- [ ] Update `book/src/api/builder.md` with MCP methods
- [ ] Add MCP example to `examples/mcp-demo/`
- [ ] Update README with MCP section

## File Count Estimate

| File | LOC (approx) |
|------|--------------|
| `lib.rs` | 30 |
| `protocol/messages.rs` | 80 |
| `protocol/types.rs` | 120 |
| `protocol/error.rs` | 50 |
| `transport/mod.rs` | 20 |
| `transport/stdio.rs` | 80 |
| `client.rs` | 150 |
| `tool_adapter.rs` | 80 |
| **Total** | **~610 lines** |

Minimal, focused, no external MCP dependencies!

## Success Criteria

1. ✅ Agent can spawn and use MCP stdio servers
2. ✅ MCP tools appear as native SDK tools
3. ✅ No external MCP crate dependencies
4. ✅ Clean, well-documented implementation
5. ✅ Working example with filesystem server

## Related Links

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [JSON-RPC 2.0 Spec](https://www.jsonrpc.org/specification)
- [MCP Reference Servers](https://github.com/modelcontextprotocol/servers)

---

**Author**: Feature planning for rust-deep-agents-sdk  
**Branch**: `feature/mcp-support`  
**Status**: Planning  
**Approach**: From-scratch implementation (no external MCP deps)
