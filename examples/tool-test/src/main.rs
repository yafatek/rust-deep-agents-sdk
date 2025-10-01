use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_macros::tool;
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig};
use std::sync::Arc;

/// Simple tool to add two numbers together
#[tool("Adds two numbers and returns the result. Use this when the user asks for mathematical addition.")]
pub fn add_numbers(a: i32, b: i32) -> i32 {
    tracing::warn!("✅ TOOL CALLED: add_numbers({}, {})", a, b);
    let result = a + b;
    tracing::warn!("✅ TOOL RESULT: {} + {} = {}", a, b, result);
    result
}

/// Tool to register a vehicle in a mock CRM
#[tool("MANDATORY: Call this function immediately when customer provides vehicle information (make, model, year). Registers or updates vehicle in CRM database.")]
pub fn register_vehicle(
    customer_id: String,
    vehicle_make: String,
    vehicle_model: String,
    vehicle_year: Option<i32>,
) -> String {
    tracing::warn!("🚗 TOOL CALLED: Registering vehicle");
    tracing::warn!("   Customer ID: {}", customer_id);
    tracing::warn!("   Vehicle: {} {} {}", vehicle_year.unwrap_or(0), vehicle_make, vehicle_model);
    
    let result = format!(
        "✅ Vehicle registered successfully: {} {} {} for customer {}",
        vehicle_year.map(|y| y.to_string()).unwrap_or_else(|| "Unknown year".to_string()),
        vehicle_make,
        vehicle_model,
        customer_id
    );
    
    tracing::warn!("✅ TOOL RESULT: {}", result);
    result
}

/// Tool to search the web
#[tool("Search the internet for information. Use this when you need current or factual information.")]
pub fn web_search(query: String, max_results: Option<u32>) -> String {
    tracing::warn!("🔍 TOOL CALLED: web_search(\"{}\")", query);
    let max = max_results.unwrap_or(5);
    
    // Mock search results
    let result = format!(
        "Found {} results for \"{}\"\n\
        1. Result about {}\n\
        2. More information on {}\n\
        3. Latest updates regarding {}",
        max, query, query, query, query
    );
    
    tracing::warn!("✅ TOOL RESULT: {} results found", max);
    result
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "warn".into()),
        )
        .init();

    println!("🚀 OpenAI Tool Invocation Test - Bug Fix Verification");
    println!("{}", "=".repeat(60));
    println!();

    // Load environment variables from .env file
    dotenv::dotenv().ok();

    // Get OpenAI API key
    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?;

    println!("✅ OpenAI API key loaded: {}...{}", 
        &api_key[..std::cmp::min(8, api_key.len())],
        &api_key[api_key.len().saturating_sub(4)..]
    );
    println!();

    // Create system prompt
    let system_prompt = r#"You are a helpful AI assistant with access to tools.

When you need to use a tool, respond ONLY with JSON in this exact format:

```json
{
  "tool_calls": [
    {
      "name": "tool_name",
      "args": {
        "parameter": "value"
      }
    }
  ]
}
```

Available tools:
- add_numbers: Adds two numbers together
- register_vehicle: Registers a vehicle in the CRM system
- web_search: Search the internet for information

When you don't need to call a tool, respond normally with text.

IMPORTANT: You MUST use tools when appropriate. For example:
- If user asks "what is 5 + 3", call add_numbers
- If user says "I have a 2021 BMW M4", call register_vehicle
- If user asks "search for rust programming", call web_search"#;

    println!("{}", "=".repeat(60));
    println!();

    // Test 1: Math tool
    println!("📝 TEST 1: Math Tool");
    println!("{}", "-".repeat(60));
    
    let tools = vec![AddNumbersTool::as_tool()];
    let openai_config = OpenAiConfig::new(api_key.clone(), "gpt-4o-mini");
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    
    let agent = ConfigurableAgentBuilder::new(system_prompt)
        .with_openai_chat(openai_config)?
        .with_tools(tools)
        .with_checkpointer(checkpointer)
        .build()?;

    let test1 = "What is 25 + 17?";
    println!("👤 User: {}", test1);
    
    let state = Arc::new(AgentStateSnapshot::default());
    let response1 = agent.handle_message(test1, state).await?;

    println!(
        "🤖 Agent: {}",
        response1.content.as_text().unwrap_or("No response")
    );
    println!();
    println!("{}", "=".repeat(60));
    println!();

    // Test 2: Vehicle registration tool
    println!("📝 TEST 2: Vehicle Registration Tool");
    println!("{}", "-".repeat(60));

    let tools = vec![RegisterVehicleTool::as_tool()];
    let openai_config = OpenAiConfig::new(api_key.clone(), "gpt-4o-mini");
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    
    let agent = ConfigurableAgentBuilder::new(system_prompt)
        .with_openai_chat(openai_config)?
        .with_tools(tools)
        .with_checkpointer(checkpointer)
        .build()?;

    let test2 = "I have a 2021 BMW M4 CS. My customer ID is CUST-12345.";
    println!("👤 User: {}", test2);

    let state = Arc::new(AgentStateSnapshot::default());
    let response2 = agent.handle_message(test2, state).await?;

    println!(
        "🤖 Agent: {}",
        response2.content.as_text().unwrap_or("No response")
    );
    println!();
    println!("{}", "=".repeat(60));
    println!();

    // Test 3: Web search tool
    println!("📝 TEST 3: Web Search Tool");
    println!("{}", "-".repeat(60));

    let tools = vec![WebSearchTool::as_tool()];
    let openai_config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    
    let agent = ConfigurableAgentBuilder::new(system_prompt)
        .with_openai_chat(openai_config)?
        .with_tools(tools)
        .with_checkpointer(checkpointer)
        .build()?;

    let test3 = "Search for information about Rust programming language";
    println!("👤 User: {}", test3);

    let state = Arc::new(AgentStateSnapshot::default());
    let response3 = agent.handle_message(test3, state).await?;

    println!(
        "🤖 Agent: {}",
        response3.content.as_text().unwrap_or("No response")
    );
    println!();
    println!("{}", "=".repeat(60));
    println!();

    println!("🎉 All tests completed!");
    println!();
    println!("✅ SUCCESS! If you see the tool logs above:");
    println!("   - '✅ TOOL CALLED: add_numbers' for Test 1");
    println!("   - '🚗 TOOL CALLED: Registering vehicle' for Test 2");
    println!("   - '🔍 TOOL CALLED: web_search' for Test 3");
    println!();
    println!("Then the bug fix is working correctly!");
    println!("OpenAI is now properly invoking tools via function calling.");

    Ok(())
}
