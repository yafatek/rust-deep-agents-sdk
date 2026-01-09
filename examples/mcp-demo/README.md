# MCP Demo

This example demonstrates how to integrate MCP (Model Context Protocol) servers with the Rust Deep Agents SDK.

## What is MCP?

Model Context Protocol (MCP) is an open standard introduced by Anthropic that allows AI systems to interact with external tools and data sources in a standardized way. It's supported by major AI providers including OpenAI, Google, and Anthropic.

## Prerequisites

1. **Node.js and npm** - Required to run the MCP filesystem server
2. **OpenAI API Key** - Set in the `OPENAI_API_KEY` environment variable

## Running the Demo

```bash
# Set your API key
export OPENAI_API_KEY="your-api-key"

# Run with default /tmp directory
cargo run -p mcp-demo

# Or specify a directory
cargo run -p mcp-demo -- /path/to/your/directory
```

## How It Works

1. **Spawn MCP Server**: The example spawns the `@modelcontextprotocol/server-filesystem` as a subprocess
2. **Connect**: Establishes a JSON-RPC connection over stdio
3. **Discover Tools**: Lists available tools from the MCP server (read_file, write_file, list_directory, etc.)
4. **Create Agent**: Builds a Deep Agent with the MCP tools
5. **Use**: The agent can now use filesystem operations to complete tasks

## Example Code

```rust
use agents_sdk::{
    create_mcp_tools, ConfigurableAgentBuilder, McpClient, StdioTransport,
    OpenAiConfig, OpenAiChatModel,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create the model
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    // Spawn MCP filesystem server
    let transport = StdioTransport::spawn(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", "/tmp"],
    ).await?;

    // Connect to MCP server
    let mcp_client = Arc::new(McpClient::connect(transport).await?);

    // Convert MCP tools to SDK tools
    let mcp_tools = create_mcp_tools(mcp_client, Some("fs"));

    // Build agent with MCP tools
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_tools(mcp_tools)
        .build()?;

    // Use the agent - it can now read/write files!
    let response = agent.handle_message(
        "List files in /tmp",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    Ok(())
}
```

## Available MCP Servers

The [Model Context Protocol](https://github.com/modelcontextprotocol/servers) project provides many pre-built servers:

- **@modelcontextprotocol/server-filesystem** - File operations
- **@modelcontextprotocol/server-github** - GitHub API integration
- **@modelcontextprotocol/server-slack** - Slack messaging
- **@modelcontextprotocol/server-postgres** - PostgreSQL queries
- And many more...

## Custom MCP Servers

You can also create custom MCP servers in any language and connect them using the same pattern:

```rust
// Connect to your custom MCP server
let transport = StdioTransport::spawn("./my-custom-mcp-server", &[]).await?;
let client = Arc::new(McpClient::connect(transport).await?);
```

## Security Notes

- The MCP filesystem server only has access to the directory you specify
- Always review the tools available before using them in production
- Consider using tool interrupts (HITL) for sensitive operations

