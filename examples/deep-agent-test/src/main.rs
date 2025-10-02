//! Deep Agent Test Example
//!
//! This example demonstrates the new automatic Deep Agent features:
//! 1. Automatic tool calling with comprehensive logging
//! 2. Sub-agent delegation via automatic task() tool
//! 3. TODO persistence across conversation turns
//! 4. Built-in Deep Agent system prompt

use agents_sdk::{
    ConfigurableAgentBuilder, OpenAiConfig, SubAgentConfig, 
    persistence::InMemoryCheckpointer, tool
};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

// Define a simple tool for testing automatic tool calling
#[tool("Adds two numbers together - use this when user asks for math")]
fn add_numbers(a: i32, b: i32) -> i32 {
    println!("🧮 ADD_NUMBERS TOOL CALLED: {} + {} = {}", a, b, a + b);
    a + b
}

// Define a tool for the calculator sub-agent
#[tool("Performs complex mathematical calculations")]
fn calculate(expression: String) -> String {
    println!("🔢 CALCULATE TOOL CALLED: {}", expression);
    
    // Simple expression evaluator (just for demo)
    if expression.contains("+") {
        let parts: Vec<&str> = expression.split('+').collect();
        if parts.len() == 2 {
            if let (Ok(a), Ok(b)) = (parts[0].trim().parse::<i32>(), parts[1].trim().parse::<i32>()) {
                return format!("{} + {} = {}", a, b, a + b);
            }
        }
    }
    
    format!("Calculated: {}", expression)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging to see all the Deep Agent activity
    tracing_subscriber::fmt()
        .with_env_filter("deep_agent_test=info,agents_runtime=warn,agents_toolkit=warn")
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();
    
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set in .env file");

    info!("🚀 Starting Deep Agent Test");
    info!("This will demonstrate:");
    info!("  1. ✅ Automatic tool calling with logging");
    info!("  2. 🎯 Sub-agent delegation");
    info!("  3. 📋 TODO persistence");
    info!("  4. 🤖 Built-in Deep Agent prompts");

    // Create OpenAI configuration
    let openai_config = OpenAiConfig::new(api_key, "gpt-4o-mini");

    // Create a calculator sub-agent
    let calculator_subagent = SubAgentConfig::new(
        "calculator",
        "Mathematical calculation specialist",
        "You are a calculator agent. Use the calculate tool to perform mathematical operations."
    ).with_tools(vec![CalculateTool::as_tool()]);

    // Create a research sub-agent (no tools, just analysis)
    let research_subagent = SubAgentConfig::new(
        "researcher", 
        "Research and analysis specialist",
        "You are a research agent. Analyze topics and provide detailed insights."
    );

    // Build the Deep Agent with automatic features
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant that can do math and research. Use your tools and sub-agents to help users."
    )
    .with_openai_chat(openai_config)?
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .with_tool(AddNumbersTool::as_tool())
    .with_subagent_config(vec![calculator_subagent, research_subagent])
    .build()?;

    info!("🤖 Deep Agent created with automatic features enabled");

    // Test 1: Automatic tool calling
    info!("\n=== TEST 1: Automatic Tool Calling ===");
    test_automatic_tool_calling(&agent).await?;

    // Test 2: Sub-agent delegation  
    info!("\n=== TEST 2: Sub-Agent Delegation ===");
    test_subagent_delegation(&agent).await?;

    // Test 3: TODO persistence
    info!("\n=== TEST 3: TODO Persistence ===");
    test_todo_persistence(&agent).await?;

    // Test 4: Multi-step workflow
    info!("\n=== TEST 4: Multi-Step Workflow ===");
    test_multistep_workflow(&agent).await?;

    info!("🎉 All Deep Agent tests completed successfully!");
    Ok(())
}

async fn test_automatic_tool_calling(agent: &agents_sdk::DeepAgent) -> Result<()> {
    info!("Testing if agent automatically calls tools...");
    
    let response = agent.handle_message(
        "What is 15 + 27?",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;
    
    info!("Response: {}", response.content.as_text().unwrap_or("No text"));
    Ok(())
}

async fn test_subagent_delegation(agent: &agents_sdk::DeepAgent) -> Result<()> {
    info!("Testing sub-agent delegation...");
    
    let response = agent.handle_message(
        "Use the calculator agent to compute 25 * 4",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;
    
    info!("Response: {}", response.content.as_text().unwrap_or("No text"));
    Ok(())
}

async fn test_todo_persistence(agent: &agents_sdk::DeepAgent) -> Result<()> {
    info!("Testing TODO persistence...");
    
    // First message: create a plan
    let response1 = agent.handle_message(
        "Create a plan to research artificial intelligence and write a summary",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;
    
    info!("Plan created: {}", response1.content.as_text().unwrap_or("No text"));
    
    // Save state
    agent.save_state(&"test-thread".to_string()).await?;
    
    // Second message: check the plan (simulating conversation continuation)
    let response2 = agent.handle_message(
        "What's my current plan?",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;
    
    info!("Plan retrieved: {}", response2.content.as_text().unwrap_or("No text"));
    Ok(())
}

async fn test_multistep_workflow(agent: &agents_sdk::DeepAgent) -> Result<()> {
    info!("Testing complex multi-step workflow...");
    
    let response = agent.handle_message(
        "I need to calculate 100 + 200, then research the history of mathematics, and create a plan for both tasks",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;
    
    info!("Workflow response: {}", response.content.as_text().unwrap_or("No text"));
    Ok(())
}