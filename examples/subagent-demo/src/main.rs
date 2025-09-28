mod tools;

use std::env;
use std::sync::Arc;

use agents_core::messaging::{AgentMessage, MessageContent};
use agents_core::state::AgentStateSnapshot;
use agents_runtime::create_deep_agent;
use agents_runtime::graph::{CreateDeepAgentParams, SubAgentConfig};
use agents_runtime::providers::openai::{OpenAiChatModel, OpenAiConfig};
use anyhow::{Context, Result};
use tools::{calculator_tool, web_search_tool, TavilyConfig};
use tracing::info;

/// Read an environment variable, returning a helpful error if missing.
fn env_var(name: &str) -> Result<String> {
    env::var(name).with_context(|| format!("Missing required environment variable {name}"))
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    // Load required configuration
    dotenv::dotenv().ok();
    let openai_key = env_var("OPENAI_API_KEY")?;
    let tavily_key = env_var("TAVILY_API_KEY")?;
    let tavily_url = env::var("TAVILY_API_URL").ok().filter(|s| !s.is_empty());
    info!("OPENAI_API_KEY loaded, starting subagent demo");

    // Configure shared OpenAI model (shared across agents for this demo)
    let openai_config = OpenAiConfig::new(openai_key.clone(), "gpt-4o-mini");

    // Create model
    let model = Arc::new(OpenAiChatModel::new(openai_config)?);

    // Create web search tool backed by Tavily
    let tavily_tool = web_search_tool(TavilyConfig {
        api_key: tavily_key,
        api_url: tavily_url,
    })?;

    let main_agent = create_deep_agent(CreateDeepAgentParams {
        instructions: "You are the primary orchestrator. Delegate tasks to the appropriate specialized subagents based on the task type.".into(),
        model: Some(model),
        subagents: vec![
            SubAgentConfig {
                name: "calculator".into(),
                description: "Perform mathematical computations".into(),
                instructions: "You are a calculator. Use the calculator tool to evaluate expressions precisely.".into(),
                tools: Some(vec![calculator_tool()]),
                planner: None,
            },
            SubAgentConfig {
                name: "searcher".into(),
                description: "Research tasks and web search".into(),
                instructions: "You are a web researcher. Use the web_search tool to find information and summarize it.".into(),
                tools: Some(vec![tavily_tool]),
                planner: None,
            },
        ],
        ..Default::default()
    })?;

    println!("=== Multi-Agent Delegation Demo ===");

    // 1) Perform a research task via the web search subagent
    let query = "current state of quantum computing";
    println!("\nUser: Research task -> {query}");
    let response = main_agent
        .handle_message(
            format!("Search the Web for the {query} and highlight key milestones from the last 3 years."),
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;
    print_agent_response(&response);

    // 2) Perform a calculation task via the calculator subagent
    let expression = "(42 * 19) / (3 + 4.5)";
    println!("\nUser: Calculation task -> {expression}");
    let response = main_agent
        .handle_message(
            format!("Compute this expression accurately: {expression}"),
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;
    print_agent_response(&response);

    println!("\n✅ Demo complete! The orchestrator delegated tasks to specialized subagents.");
    Ok(())
}

fn print_agent_response(message: &AgentMessage) {
    match &message.content {
        MessageContent::Text(text) => {
            println!("Assistant: {text}");
        }
        MessageContent::Json(value) => {
            println!("Assistant (json): {}", value);
        }
    }
}
