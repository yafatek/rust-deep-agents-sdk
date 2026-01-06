# MCP (Model Context Protocol) Integration Plan

## Overview

This document outlines the plan to add MCP support to the Rust Deep Agents SDK, enabling agents to use external MCP servers as tool providers.

## What is MCP?

**Model Context Protocol (MCP)** is an open standard introduced by Anthropic in November 2024 to standardize how AI systems interact with external tools, data sources, and services. It provides:

- **Universal Interface**: Standardized way for LLMs to call tools
- **Transport Agnostic**: Works over stdio, HTTP/SSE, WebSocket
- **Industry Adoption**: OpenAI, Google DeepMind, and Anthropic all support it

### MCP Core Concepts

| Concept | Description |
|---------|-------------|
| **Tools** | Functions that can be called by the AI model |
| **Resources** | Data sources the AI can read (files, databases, etc.) |
| **Prompts** | Pre-defined prompt templates |
| **Sampling** | Request completions from the AI model |

## Integration Goals

### Primary Goal
Enable Deep Agents to **consume tools from MCP servers** as first-class tool providers.

```rust
// Future API Vision
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_mcp_server("npx @anthropic-ai/mcp-server-filesystem")  // MCP server
    .with_mcp_server("http://localhost:8080/mcp")                 // HTTP MCP
    .with_tool(MyCustomTool::as_tool())                          // Regular tools
    .build()?;
```

### Secondary Goals
1. Allow converting SDK tools to MCP-compatible format (exposing as MCP server)
2. Support MCP resources and prompts
3. Maintain backward compatibility with existing tool API

## Candidate Crates

### Option 1: `rust-mcp-sdk` (Recommended)
- **Version**: 0.8.1
- **Features**: Full MCP client & server, async/Tokio, multiple transports
- **License**: MIT
- **Pros**: Most complete, well-maintained, official ecosystem
- **Cons**: Heavy feature set (but feature-gated)

### Option 2: `rmcp-agent`
- **Version**: 0.1.6  
- **Features**: LangChain-style integration, tool conversion
- **License**: MIT
- **Pros**: Designed for agent frameworks like ours
- **Cons**: Less mature, fewer features

### Option 3: `mcp_client_rs`
- **Version**: 0.1.7
- **Features**: Client-only implementation
- **License**: MIT
- **Pros**: Lightweight
- **Cons**: No server support, less active

**Recommendation**: Use `rust-mcp-sdk` for the core protocol, potentially reference `rmcp-agent` for integration patterns.

## Architecture Design

### New Crate: `agents-mcp`

```
crates/
├── agents-mcp/              # New crate
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── client.rs        # MCP client wrapper
│       ├── tool_adapter.rs  # Convert MCP tools to SDK tools
│       ├── server.rs        # Expose SDK as MCP server (future)
│       └── transport.rs     # Transport configuration
```

### Integration Points

```
┌─────────────────────────────────────────────────────────────┐
│                    DeepAgent                                 │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐                   │
│  │  SDK Tools      │  │  MCP Tools      │                   │
│  │  (ToolBox)      │  │  (McpToolBox)   │ ◄── NEW           │
│  └────────┬────────┘  └────────┬────────┘                   │
│           │                    │                             │
│           ▼                    ▼                             │
│  ┌─────────────────────────────────────────────────┐        │
│  │              Unified Tool Registry              │        │
│  │  - Native tools                                 │        │
│  │  - MCP server tools (dynamically loaded)        │        │
│  └─────────────────────────────────────────────────┘        │
└─────────────────────────────────────────────────────────────┘
                           │
                           ▼
        ┌─────────────────────────────────────┐
        │         MCP Servers                  │
        ├─────────────────────────────────────┤
        │  - Filesystem server                 │
        │  - Database server                   │
        │  - Custom business logic servers     │
        │  - Third-party integrations          │
        └─────────────────────────────────────┘
```

### API Design

#### Builder Extension

```rust
impl ConfigurableAgentBuilder {
    /// Add an MCP server via stdio (subprocess)
    pub fn with_mcp_stdio(self, command: &str, args: &[&str]) -> Self
    
    /// Add an MCP server via HTTP/SSE
    pub fn with_mcp_http(self, url: &str) -> Self
    
    /// Add an MCP server with full configuration
    pub fn with_mcp_server(self, config: McpServerConfig) -> Self
}
```

#### MCP Server Configuration

```rust
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    pub tools_filter: Option<Vec<String>>,  // Only expose certain tools
    pub timeout: Duration,
}

pub enum McpTransport {
    Stdio { command: String, args: Vec<String>, env: HashMap<String, String> },
    Http { url: String, headers: HashMap<String, String> },
    WebSocket { url: String },
}
```

#### Tool Adapter

```rust
/// Wraps an MCP tool to implement our Tool trait
pub struct McpTool {
    server: Arc<McpClient>,
    tool_info: McpToolInfo,
}

#[async_trait]
impl Tool for McpTool {
    fn schema(&self) -> ToolSchema {
        // Convert MCP tool schema to SDK schema
    }
    
    async fn execute(&self, args: Value, ctx: ToolContext) -> anyhow::Result<ToolResult> {
        // Forward to MCP server
        self.server.call_tool(&self.tool_info.name, args).await
    }
}
```

## Implementation Phases

### Phase 1: Core MCP Client (MVP)
**Estimated: 2-3 days**

- [ ] Create `agents-mcp` crate
- [ ] Add `rust-mcp-sdk` dependency
- [ ] Implement `McpClient` wrapper
- [ ] Implement `McpTool` adapter
- [ ] Add `with_mcp_stdio()` to builder
- [ ] Basic example with filesystem MCP server

### Phase 2: HTTP/SSE Transport
**Estimated: 1-2 days**

- [ ] Add HTTP transport support
- [ ] Add SSE streaming support
- [ ] Connection pooling and retry logic
- [ ] `with_mcp_http()` builder method

### Phase 3: Advanced Features
**Estimated: 2-3 days**

- [ ] MCP Resources support (read data sources)
- [ ] MCP Prompts support (template management)
- [ ] Tool filtering and namespacing
- [ ] Health checks and reconnection

### Phase 4: Server Mode (Optional)
**Estimated: 3-4 days**

- [ ] Expose SDK tools as MCP server
- [ ] Support for SDK as MCP provider
- [ ] Documentation and examples

## Example Usage (Target API)

```rust
use agents_sdk::{
    ConfigurableAgentBuilder, 
    OpenAiConfig, 
    OpenAiChatModel,
    mcp::McpServerConfig,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant with filesystem access.")
        .with_model(model)
        // Add MCP filesystem server (runs as subprocess)
        .with_mcp_stdio("npx", &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"])
        // Add custom MCP server via HTTP
        .with_mcp_http("http://localhost:8080/mcp")
        // Can still use native tools
        .with_tool(MyCustomTool::as_tool())
        .build()?;

    // Agent now has access to:
    // - Filesystem tools from MCP server (read_file, write_file, list_directory, etc.)
    // - HTTP server tools
    // - Native MyCustomTool
    
    let response = agent.handle_message(
        "List the files in /tmp and read the first text file",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());
    Ok(())
}
```

## Dependencies

```toml
# crates/agents-mcp/Cargo.toml
[dependencies]
agents-core = { path = "../agents-core" }
rust-mcp-sdk = { version = "0.8", default-features = false, features = ["client", "stdio", "sse"] }
tokio = { version = "1.0", features = ["process", "io-util"] }
async-trait = "0.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
tracing = "0.1"
anyhow = "1.0"
```

## Testing Strategy

1. **Unit Tests**: Mock MCP responses, test tool conversion
2. **Integration Tests**: Real MCP server (filesystem) in CI
3. **Example Programs**: Demonstrate various MCP servers

## Documentation Updates

- [ ] Add `book/src/features/mcp.md`
- [ ] Update `book/src/api/builder.md` with MCP methods
- [ ] Add MCP example to `examples/`
- [ ] Update README with MCP section

## Risks & Mitigations

| Risk | Mitigation |
|------|------------|
| MCP spec changes | Pin to specific schema version |
| Performance overhead | Connection pooling, async execution |
| Complex error handling | Wrap MCP errors in SDK error types |
| Breaking changes | Feature-gate behind `mcp` flag |

## Success Criteria

1. ✅ Agent can use tools from MCP stdio server
2. ✅ Agent can use tools from MCP HTTP server  
3. ✅ Existing tool API unchanged
4. ✅ Comprehensive documentation
5. ✅ At least one working example

## Related Links

- [MCP Specification](https://spec.modelcontextprotocol.io/)
- [MCP GitHub](https://github.com/modelcontextprotocol)
- [rust-mcp-sdk](https://github.com/rust-mcp-stack/rust-mcp-sdk)
- [Discussion: MCP Support](https://github.com/yafatek/rust-deep-agents-sdk/discussions) - @bbigras request

## Timeline

| Week | Milestone |
|------|-----------|
| Week 1 | Phase 1 complete (MVP) |
| Week 2 | Phase 2 complete (HTTP support) |
| Week 3 | Phase 3 complete (Advanced features) |
| Week 4 | Documentation, testing, PR review |

---

**Author**: Feature planning for rust-deep-agents-sdk  
**Branch**: `feature/mcp-support`  
**Status**: Planning

