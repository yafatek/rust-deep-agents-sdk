//! # HTTP MCP Demo
//!
//! Demonstrates using HTTP-based MCP servers with the Deep Agents SDK.
//!
//! ## Running
//!
//! ```bash
//! # Set your OpenAI API key
//! export OPENAI_API_KEY=sk-your-key
//!
//! # Run the demo
//! cargo run -p http-mcp-demo
//! ```

use agents_sdk::{
    create_mcp_tools, state::AgentStateSnapshot, AnthropicConfig, AnthropicMessagesModel,
    ConfigurableAgentBuilder, HttpTransport, McpClient, OpenAiChatModel, OpenAiConfig,
    SubAgentConfig,
};
use std::io::{self, BufRead, Write};
use std::sync::Arc;
use tracing::{error, info, Level};
use tracing_subscriber::FmtSubscriber;

/// MCP Server configuration
struct McpServerConfig {
    url: String,
    name: String,
}

/// LLM Provider enum - supports Anthropic (fast) or OpenAI
enum LlmProvider {
    Anthropic(Arc<AnthropicMessagesModel>),
    OpenAi(Arc<OpenAiChatModel>),
}

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
    println!("║       🌐 Deep Agents SDK - HTTP MCP Demo                     ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Demonstrates HTTP-based MCP server integration              ║");
    println!("║  with Deep Agent architecture                                ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    // Check for API keys - prefer Anthropic for speed
    let anthropic_key = std::env::var("ANTHROPIC_API_KEY").ok();
    let openai_key = std::env::var("OPENAI_API_KEY").ok();

    let provider = if let Some(api_key) = anthropic_key {
        let config = AnthropicConfig::new(api_key, "claude-haiku-4-5-20251001", 4096);
        info!("✅ Using Anthropic Claude Haiku (FAST!)");
        LlmProvider::Anthropic(Arc::new(AnthropicMessagesModel::new(config)?))
    } else if let Some(api_key) = openai_key {
        let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
        info!("✅ Using OpenAI gpt-4o-mini");
        LlmProvider::OpenAi(Arc::new(OpenAiChatModel::new(config)?))
    } else {
        return Err(anyhow::anyhow!(
            "No API key found!\n\
             Set ANTHROPIC_API_KEY (recommended for speed) or OPENAI_API_KEY in .env"
        ));
    };

    // =========================================
    // Configure MCP Server
    // =========================================
    // You can configure any HTTP-based MCP server here
    let mcp_config = McpServerConfig {
        url: std::env::var("MCP_SERVER_URL")
            .unwrap_or_else(|_| "https://mcp.context7.com/mcp".to_string()),
        name: "docs".to_string(),
    };

    info!("📡 Connecting to MCP server: {}", mcp_config.url);

    // =========================================
    // Connect via HTTP Transport
    // =========================================
    let transport = HttpTransport::new(&mcp_config.url)
        // Add any required headers (e.g., authentication)
        // .with_header("Authorization", "Bearer your-token")
        .with_timeout_secs(30)
        .build()?;

    let mcp_client = match McpClient::connect(transport).await {
        Ok(client) => {
            info!("✅ Connected to MCP server");
            Arc::new(client)
        }
        Err(e) => {
            error!("Failed to connect to MCP server: {}", e);
            println!("\n⚠️  Could not connect to MCP server at {}", mcp_config.url);
            println!("    The demo will continue without MCP tools.\n");
            println!("    To use MCP tools, set MCP_SERVER_URL environment variable");
            println!("    to a valid HTTP MCP server endpoint.\n");

            // Continue without MCP tools for demo purposes
            return run_without_mcp(provider).await;
        }
    };

    // List discovered tools
    println!("\n🔧 Discovered MCP Tools:");
    let tools = mcp_client.tools();
    if tools.is_empty() {
        println!("   No tools discovered from server");
    } else {
        for tool in tools {
            println!(
                "   • {} — {}",
                tool.name,
                tool.description.as_deref().unwrap_or("No description")
            );
        }
    }

    // Convert MCP tools to SDK tools
    let mcp_tools = create_mcp_tools(mcp_client.clone(), Some(&mcp_config.name));
    info!(
        "✅ {} MCP tools registered with '{}.' prefix",
        mcp_tools.len(),
        mcp_config.name
    );

    // =========================================
    // Define Sub-Agents
    // =========================================
    let research_agent = SubAgentConfig::new(
        "research-agent",
        "Specialist for in-depth research and analysis. Delegate when you need \
         comprehensive information gathering or detailed technical analysis.",
        "You are a research specialist. Your job is to thoroughly investigate topics, \
         gather information from available tools, and provide comprehensive, well-structured \
         responses. Always cite your sources and be clear about what you found.",
    );

    let summary_agent = SubAgentConfig::new(
        "summary-agent",
        "Specialist for creating concise summaries. Delegate when you need to \
         condense large amounts of information into digestible formats.",
        "You are a summarization expert. Your job is to take complex information \
         and distill it into clear, concise summaries. Focus on key points and \
         actionable insights.",
    );

    // =========================================
    // Build the Main Agent
    // =========================================
    let system_prompt = format!(
        r#"You are a Deep Agent research assistant with access to documentation lookup tools.

Your capabilities:
1. **Documentation Lookup** — You have MCP tools (prefixed with '{}') for looking up technical documentation
2. **Sub-Agent Delegation** — You can delegate to specialized agents:
   - `research-agent`: For comprehensive research tasks
   - `summary-agent`: For creating summaries

When handling requests:
1. Use the documentation tools to find accurate, up-to-date information
2. Delegate complex tasks to appropriate sub-agents
3. Always provide clear, well-structured responses

Be helpful, accurate, and cite your sources when providing technical information."#,
        mcp_config.name
    );

    let agent = match provider {
        LlmProvider::Anthropic(model) => ConfigurableAgentBuilder::new(&system_prompt)
            .with_model(model)
            .with_tools(mcp_tools)
            .with_subagent_config([research_agent, summary_agent])
            .with_max_iterations(10)
            .build()?,
        LlmProvider::OpenAi(model) => ConfigurableAgentBuilder::new(&system_prompt)
            .with_model(model)
            .with_tools(mcp_tools)
            .with_subagent_config([research_agent, summary_agent])
            .with_max_iterations(10)
            .build()?,
    };

    info!("✅ Deep Agent built with MCP tools and sub-agents");

    // =========================================
    // Interactive Chat Loop
    // =========================================
    run_chat_loop(agent).await
}

/// Run the agent without MCP tools (fallback)
async fn run_without_mcp(provider: LlmProvider) -> anyhow::Result<()> {
    let prompt = "You are a helpful assistant. Note: Documentation lookup tools are not available \
         in this session. Please provide general guidance based on your knowledge.";

    let agent = match provider {
        LlmProvider::Anthropic(model) => ConfigurableAgentBuilder::new(prompt)
            .with_model(model)
            .with_max_iterations(5)
            .build()?,
        LlmProvider::OpenAi(model) => ConfigurableAgentBuilder::new(prompt)
            .with_model(model)
            .with_max_iterations(5)
            .build()?,
    };

    run_chat_loop(agent).await
}

/// Run the interactive chat loop
async fn run_chat_loop(
    agent: agents_sdk::DeepAgent,
) -> anyhow::Result<()> {
    println!("\n╔══════════════════════════════════════════════════════════════╗");
    println!("║  🎯 Agent Ready! Try these example queries:                  ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  • \"How do I create a web server in Rust?\"                  ║");
    println!("║  • \"What's the latest AWS provider for Terraform?\"          ║");
    println!("║  • \"Best practices for building a REST API\"                 ║");
    println!("║  • \"How to use async/await in Rust?\"                        ║");
    println!("╠══════════════════════════════════════════════════════════════╣");
    println!("║  Type 'quit' or 'exit' to end the session                    ║");
    println!("╚══════════════════════════════════════════════════════════════╝");
    println!();

    let stdin = io::stdin();
    let state = Arc::new(AgentStateSnapshot::default());

    loop {
        print!("You: ");
        io::stdout().flush()?;

        let mut input = String::new();
        stdin.lock().read_line(&mut input)?;
        let input = input.trim();

        if input.is_empty() {
            continue;
        }
        if input.eq_ignore_ascii_case("quit") || input.eq_ignore_ascii_case("exit") {
            println!("\n👋 Goodbye!");
            break;
        }

        println!();
        let start = std::time::Instant::now();
        info!("Processing... (timing external APIs)");

        match agent.handle_message(input, state.clone()).await {
            Ok(response) => {
                let elapsed = start.elapsed();
                println!("\n🤖 Agent (took {:.1}s - mostly OpenAI/MCP API latency):", elapsed.as_secs_f64());
                println!("{}", response.content.as_text().unwrap_or("No response"));
                println!();
            }
            Err(e) => {
                error!("Error: {}", e);
                println!("\n❌ Error: {}\n", e);
            }
        }
    }

    Ok(())
}
