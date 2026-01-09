# MCP (Model Context Protocol) Integration

The Rust Deep Agents SDK supports **MCP (Model Context Protocol)** — a standard protocol for connecting AI models to external tools and data sources.

## What is MCP?

[Model Context Protocol](https://modelcontextprotocol.io/) is an open protocol that enables seamless integration between AI applications and external tools. With MCP, you can:

- Connect to **MCP servers** that provide tools (filesystem, databases, APIs)
- Use tools from any language (Python, Node.js, Go) in your Rust agents
- Access rich ecosystems of pre-built MCP servers

## Why MCP for Deep Agents?

MCP extends the Deep Agents architecture by:

| Benefit | Description |
|---------|-------------|
| **External Tools** | Access tools hosted on MCP servers without reimplementing them |
| **Language Agnostic** | Use Python or Node.js MCP servers from Rust |
| **Rich Ecosystems** | Tap into the growing MCP server ecosystem |
| **Context Management** | MCP servers can manage large datasets efficiently |

This aligns with Deep Agents' focus on extensible tool systems and context management.

## Quick Start

### Installation

Enable the `mcp` feature:

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["mcp"] }
```

Or with all features:

```toml
agents-sdk = { version = "0.0.29", features = ["full"] }
```

### Basic Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel,
    McpClient, StdioTransport, create_mcp_tools,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure the LLM
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o-mini"
    );
    let model = Arc::new(OpenAiChatModel::new(config)?);

    // Spawn an MCP filesystem server
    let transport = StdioTransport::spawn(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
    ).await?;

    // Connect to the MCP server
    let mcp_client = Arc::new(McpClient::connect(transport).await?);

    // List discovered tools
    println!("Available MCP tools:");
    for tool in mcp_client.tools() {
        println!("  - {}: {}", 
            tool.name, 
            tool.description.as_deref().unwrap_or("No description")
        );
    }

    // Convert MCP tools to SDK tools with namespace
    let mcp_tools = create_mcp_tools(mcp_client.clone(), Some("fs"));

    // Build an agent with MCP tools
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant with filesystem access. 
         You can read, write, and list files in /tmp."
    )
    .with_model(model)
    .with_tools(mcp_tools)
    .build()?;

    // Use the agent
    let response = agent.handle_message(
        "List all files in /tmp",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("Response: {}", response.content.as_text().unwrap_or("No response"));
    Ok(())
}
```

## MCP Client API

### Connecting to Servers

#### Stdio Transport (Subprocess)

The most common way to connect to MCP servers:

```rust
use agents_sdk::{McpClient, StdioTransport};

// Basic spawn
let transport = StdioTransport::spawn("npx", &[
    "-y", 
    "@modelcontextprotocol/server-filesystem", 
    "/path/to/directory"
]).await?;

let client = McpClient::connect(transport).await?;
```

#### With Configuration

```rust
use agents_sdk::{McpClient, McpClientConfig, StdioTransport};
use std::time::Duration;

let config = McpClientConfig {
    request_timeout: Duration::from_secs(60),
    auto_list_tools: true,
    client_name: Some("my-agent".to_string()),
    client_version: Some("1.0.0".to_string()),
};

let transport = StdioTransport::spawn("python", &["my_mcp_server.py"]).await?;
let client = McpClient::connect_with_config(transport, config).await?;
```

### Working with Tools

#### Listing Tools

```rust
// Get all tools
let tools = client.tools();

// Check if a tool exists
if client.has_tool("read_file") {
    println!("read_file is available!");
}

// Get a specific tool
if let Some(tool) = client.get_tool("read_file") {
    println!("Description: {:?}", tool.description);
}
```

#### Calling Tools Directly

```rust
use serde_json::json;

// Call a tool with JSON arguments
let result = client.call_tool("read_file", json!({
    "path": "/tmp/example.txt"
})).await?;

// Check for errors
if result.is_error {
    eprintln!("Tool error!");
}

// Process content
for content in &result.content {
    match content {
        McpContent::Text { text } => println!("Text: {}", text),
        McpContent::Image { data, mime_type } => {
            println!("Image: {} ({} bytes)", mime_type, data.len());
        }
        McpContent::Resource { uri, text, .. } => {
            println!("Resource: {} - {:?}", uri, text);
        }
    }
}
```

### Integrating with Agents

#### Using `create_mcp_tools`

The `create_mcp_tools` function converts all MCP tools to SDK tools:

```rust
use agents_sdk::create_mcp_tools;

// Without namespace (original tool names)
let tools = create_mcp_tools(client.clone(), None);
// Tools: read_file, write_file, list_directory, etc.

// With namespace (prefixed tool names)
let tools = create_mcp_tools(client.clone(), Some("fs"));
// Tools: fs.read_file, fs.write_file, fs.list_directory, etc.
```

Namespacing is recommended when using multiple MCP servers to avoid name collisions.

#### Adding MCP Tools to an Agent

```rust
let agent = ConfigurableAgentBuilder::new("Your instructions here")
    .with_model(model)
    .with_tools(mcp_tools)  // Add MCP tools
    .with_tool(MyNativeTool::as_tool())  // Can mix with native tools
    .build()?;
```

## Available MCP Servers

Many MCP servers are available. Here are some popular ones:

| Server | Install | Description |
|--------|---------|-------------|
| **Filesystem** | `npx @modelcontextprotocol/server-filesystem <path>` | File operations |
| **GitHub** | `npx @modelcontextprotocol/server-github` | GitHub API |
| **Postgres** | `npx @modelcontextprotocol/server-postgres <conn>` | PostgreSQL queries |
| **Brave Search** | `npx @anthropic/mcp-server-brave-search` | Web search |

See the [MCP Servers Directory](https://github.com/modelcontextprotocol/servers) for more options.

## Error Handling

The MCP client provides comprehensive error types:

```rust
use agents_sdk::McpError;

match client.call_tool("read_file", args).await {
    Ok(result) => { /* handle success */ }
    Err(McpError::Timeout(duration)) => {
        eprintln!("Request timed out after {:?}", duration);
    }
    Err(McpError::ProcessExited) => {
        eprintln!("MCP server process died");
    }
    Err(McpError::ServerError(e)) => {
        eprintln!("Server error: {}", e);
    }
    Err(e) => {
        eprintln!("Other error: {}", e);
    }
}
```

## Best Practices

### 1. Use Namespaces for Multiple Servers

```rust
let fs_tools = create_mcp_tools(fs_client.clone(), Some("fs"));
let db_tools = create_mcp_tools(db_client.clone(), Some("db"));
let api_tools = create_mcp_tools(api_client.clone(), Some("api"));

let all_tools = [fs_tools, db_tools, api_tools].concat();
```

### 2. Handle Server Lifecycle

```rust
// Store client reference to keep server alive
struct MyAgent {
    agent: DeepAgent,
    mcp_clients: Vec<Arc<McpClient>>,  // Keep alive
}

// Clients are cleaned up when dropped (kill_on_drop)
```

### 3. Configure Appropriate Timeouts

```rust
let config = McpClientConfig {
    request_timeout: Duration::from_secs(120),  // For long operations
    ..Default::default()
};
```

### 4. Check Server Capabilities

```rust
if let Some(server_info) = client.server_info() {
    if server_info.capabilities.tools.is_some() {
        // Server supports tools
    }
    if server_info.capabilities.resources.is_some() {
        // Server supports resources
    }
}
```

## Limitations

Current limitations of the MCP integration:

- **Stdio transport only** — HTTP/SSE transport not yet implemented
- **Tools only** — Resources and Prompts not yet exposed
- **Single server per client** — No built-in multiplexing

These may be addressed in future releases.

## Example: Full MCP Demo

See the complete example at [`examples/mcp-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/mcp-demo) which demonstrates:

- Spawning an MCP filesystem server
- Discovering available tools
- Creating an agent with MCP tools
- Handling user queries with file operations

```bash
# Run the demo
export OPENAI_API_KEY="your-key"
cargo run -p mcp-demo -- /tmp
```

## Related

- [Tools](../concepts/tools.md) — SDK tool system overview
- [Sub-Agents](./sub-agents.md) — Delegating work to specialized agents
- [MCP Specification](https://spec.modelcontextprotocol.io/) — Official protocol spec
