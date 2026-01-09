//! # MCP + Sub-Agents Demo
//!
//! This example demonstrates the Deep Agents architecture with:
//! - MCP integration for external tools (filesystem server)
//! - Sub-agents for specialized tasks
//! - OpenAI gpt-4o-mini as the LLM
//!
//! ## Running
//!
//! ```bash
//! # Set your OpenAI API key in .env
//! cargo run -p mcp-subagent-demo
//!
//! # Or specify a directory
//! cargo run -p mcp-subagent-demo -- /tmp
//! ```

use agents_sdk::{
    create_mcp_tools, state::AgentStateSnapshot, ConfigurableAgentBuilder, McpClient,
    OpenAiChatModel, OpenAiConfig, StdioTransport, SubAgentConfig,
};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load .env file
    dotenv::dotenv().ok();

    // Initialize tracing
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .with_target(false)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    println!();
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║       🤖 Deep Agents SDK - MCP + Sub-Agents Demo            ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  This demonstrates:                                          ║");
    println!("║  • MCP integration with filesystem server                    ║");
    println!("║  • Sub-agent delegation                                      ║");
    println!("║  • Deep Agent planning and execution                         ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Get directory to access (default to current directory)
    let directory = std::env::args().nth(1).unwrap_or_else(|| {
        std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|_| ".".to_string())
    });

    info!("📁 Filesystem access directory: {}", directory);

    // Check for API key
    let api_key = std::env::var("OPENAI_API_KEY").map_err(|_| {
        anyhow::anyhow!(
            "OPENAI_API_KEY not set!\n\
             Please create a .env file with:\n\
             OPENAI_API_KEY=sk-your-key-here"
        )
    })?;

    // Create the language model
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);
    info!("✅ OpenAI model initialized (gpt-4o-mini)");

    // =========================================
    // Step 1: Connect to MCP Filesystem Server
    // =========================================
    info!("📡 Spawning MCP filesystem server...");

    let transport = StdioTransport::spawn(
        "npx",
        &["-y", "@modelcontextprotocol/server-filesystem", &directory],
    )
    .await?;

    let mcp_client = Arc::new(McpClient::connect(transport).await?);

    // List discovered tools
    println!("\n🔧 Discovered MCP Tools:");
    for tool in mcp_client.tools() {
        println!(
            "   • {} — {}",
            tool.name,
            tool.description.as_deref().unwrap_or("No description")
        );
    }

    // Convert MCP tools to SDK tools with "fs" namespace
    let mcp_tools = create_mcp_tools(mcp_client.clone(), Some("fs"));
    info!(
        "✅ {} MCP tools registered with 'fs.' prefix",
        mcp_tools.len()
    );

    // =========================================
    // Step 2: Define Sub-Agents
    // =========================================

    // Filesystem specialist sub-agent
    let filesystem_agent = SubAgentConfig::new(
        "filesystem-agent",
        "Specialist for file operations. Use this agent when you need to read, \
         write, list, or search files. It has access to the filesystem via MCP tools.",
        "You are a filesystem specialist. Your job is to perform file operations \
         efficiently and report results clearly. When listing files, provide a \
         clean summary. When reading files, extract key information. When writing \
         files, confirm the operation was successful.",
    );

    // Research/analysis sub-agent
    let research_agent = SubAgentConfig::new(
        "research-agent",
        "Specialist for analysis and synthesis. Use this agent when you need to \
         analyze content, summarize information, or create reports from gathered data.",
        "You are a research analyst. Your job is to analyze information, identify \
         patterns, and create clear summaries. Focus on extracting actionable insights \
         and presenting them in a structured format.",
    );

    info!("✅ Sub-agents configured: filesystem-agent, research-agent");

    // =========================================
    // Step 3: Build the Main Orchestrator Agent
    // =========================================

    let agent = ConfigurableAgentBuilder::new(format!(
        r#"You are a Deep Agent orchestrator with access to a filesystem at: {}

Your capabilities:
1. **Filesystem Operations** — You have MCP tools (prefixed with 'fs.') for file operations
2. **Sub-Agent Delegation** — You can delegate to specialized agents:
   - `filesystem-agent`: For complex file operations
   - `research-agent`: For analysis and synthesis tasks

When handling requests:
1. Break down complex tasks into steps
2. Use the appropriate tools or delegate to sub-agents
3. Synthesize results into a clear response

Available filesystem tools:
- fs.read_file: Read contents of a file
- fs.write_file: Write content to a file  
- fs.list_directory: List files in a directory
- fs.search_files: Search for files by pattern
- fs.get_file_info: Get metadata about a file

Always explain what you're doing and summarize the results clearly."#,
        directory
    ))
    .with_model(model)
    .with_tools(mcp_tools)
    .with_subagent_config([filesystem_agent, research_agent])
    .with_max_iterations(15)
    .build()?;

    info!("✅ Main orchestrator agent built");

    // =========================================
    // Step 4: Interactive Chat Loop
    // =========================================

    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  🎯 Agent Ready! Try these example queries:                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  • \"List all files in this directory\"                       ║");
    println!("║  • \"Read the README.md file and summarize it\"               ║");
    println!("║  • \"Create a file called notes.txt with a todo list\"        ║");
    println!("║  • \"Find all Rust files and count them\"                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Type 'quit' or 'exit' to end the session                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let stdin = io::stdin();
    let state = Arc::new(AgentStateSnapshot::default());

    loop {
        // Print prompt
        print!("You: ");
        io::stdout().flush()?;

        // Read input
        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        // Check for exit
        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\n👋 Goodbye!");
            break;
        }

        // Process the message
        println!();
        info!("Processing request...");

        match agent.handle_message(input, state.clone()).await {
            Ok(response) => {
                // Print response
                println!("\n🤖 Agent:");
                println!("{}", response.content.as_text().unwrap_or("No response"));
                println!();
            }
            Err(e) => {
                error!("Error: {}", e);
                println!("\n❌ Error: {}\n", e);
            }
        }
    }

    // Cleanup
    info!("Cleaning up MCP connection...");
    // MCP client will be cleaned up on drop (kill_on_drop)

    Ok(())
}
