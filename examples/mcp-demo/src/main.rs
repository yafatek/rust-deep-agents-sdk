//! MCP (Model Context Protocol) Demo
//!
//! This example demonstrates how to use MCP servers with the Rust Deep Agents SDK.
//!
//! ## Prerequisites
//!
//! 1. Node.js and npm installed
//! 2. OpenAI API key set in OPENAI_API_KEY environment variable
//!
//! ## Running the Demo
//!
//! ```bash
//! # Run with default filesystem access
//! cargo run -p mcp-demo
//!
//! # Or specify a directory to access
//! cargo run -p mcp-demo -- /path/to/directory
//! ```

use agents_sdk::{
    create_mcp_tools, state::AgentStateSnapshot, ConfigurableAgentBuilder, McpClient,
    OpenAiChatModel, OpenAiConfig, StdioTransport,
};
use std::sync::Arc;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    // Get directory to access (default to /tmp for safety)
    let directory = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp".to_string());

    info!("MCP Demo - Filesystem Access");
    info!("============================");
    info!("Directory: {}", directory);

    // Check for API key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable not set"))?;

    // Create the language model
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);

    info!("\n📡 Connecting to MCP filesystem server...");

    // Spawn the MCP filesystem server
    // This uses the official @modelcontextprotocol/server-filesystem package
    let transport = StdioTransport::spawn(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", &directory],
    )
    .await?;

    // Connect to the MCP server
    let mcp_client = Arc::new(McpClient::connect(transport).await?);

    // List discovered tools
    info!("\n🔧 Discovered MCP Tools:");
    for tool in mcp_client.tools() {
        info!(
            "  - {} : {}",
            tool.name,
            tool.description.as_deref().unwrap_or("No description")
        );
    }

    // Convert MCP tools to SDK tools with a namespace prefix
    let mcp_tools = create_mcp_tools(mcp_client.clone(), Some("fs"));

    info!("\n🤖 Building agent with {} MCP tools...", mcp_tools.len());

    // Build an agent with MCP tools
    let agent = ConfigurableAgentBuilder::new(format!(
        "You are a helpful assistant with filesystem access to {}. \
         You can read, write, and list files in this directory. \
         Always confirm with the user before making changes to files.",
        directory
    ))
    .with_model(model)
    .with_tools(mcp_tools)
    .build()?;

    info!("✅ Agent ready!\n");

    // Demo: List files in the directory
    let query = format!("List all files in the {} directory", directory);
    info!("📝 User: {}", query);

    let response = agent
        .handle_message(&query, Arc::new(AgentStateSnapshot::default()))
        .await?;

    info!(
        "\n🤖 Assistant: {}",
        response.content.as_text().unwrap_or("No response")
    );

    // Cleanup: Close MCP client (optional, happens on drop)
    info!("\n✨ Demo complete!");

    Ok(())
}
