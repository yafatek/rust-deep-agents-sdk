//! Simple Deep Agent example using OpenAI GPT-4o-mini
//! 
//! This example shows how to create a basic Deep Agent using the Rust SDK
//! with OpenAI's GPT-4o-mini model, demonstrating the Python SDK equivalence:
//! 
//! Python equivalent:
//! ```python
//! from deepagents import create_deep_agent
//! from langchain_openai import ChatOpenAI
//! 
//! model = ChatOpenAI(model="gpt-4o-mini", temperature=0.7)
//! agent = create_deep_agent(
//!     tools=[internet_search],
//!     instructions="You are an expert researcher...",
//!     model=model,
//! )
//! 
//! result = agent.invoke({"messages": [{"role": "user", "content": "what is langgraph?"}]})
//! ```

use std::sync::Arc;
use agents_core::agent::AgentHandle;
use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_runtime::ConfigurableAgentBuilder;
use agents_runtime::providers::OpenAiConfig;
use agents_toolkit::create_tool;
use serde_json::Value;
use serde::{Deserialize, Serialize};

// Tavily API structures
#[derive(Serialize)]
struct TavilyRequest {
    api_key: String,
    query: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_results: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    include_raw_content: Option<bool>,
}

#[derive(Deserialize)]
struct TavilyResponse {
    results: Vec<TavilyResult>,
}

#[derive(Deserialize)]
struct TavilyResult {
    title: String,
    url: String,
    content: String,
    #[serde(default)]
    score: f64,
}

// Function to call Tavily API
async fn call_tavily_search(query: &str, max_results: Option<u32>) -> anyhow::Result<String> {
    let api_key = std::env::var("TAVILY_API_KEY")
        .map_err(|_| anyhow::anyhow!("TAVILY_API_KEY environment variable is required"))?;

    let client = reqwest::Client::new();
    let request = TavilyRequest {
        api_key,
        query: query.to_string(),
        max_results,
        include_raw_content: Some(true),
    };

    let response = client
        .post("https://api.tavily.com/search")
        .json(&request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow::anyhow!("Tavily API error: {}", response.status()));
    }

    let tavily_response: TavilyResponse = response.json().await?;
    
    // Format the results nicely
    let mut formatted_results = String::new();
    formatted_results.push_str(&format!("🔍 Search Results for '{}'\n\n", query));
    
    for (i, result) in tavily_response.results.iter().enumerate() {
        formatted_results.push_str(&format!(
            "{}. **{}**\n   URL: {}\n   Content: {}\n   Score: {:.2}\n\n",
            i + 1,
            result.title,
            result.url,
            result.content.chars().take(200).collect::<String>() + "...",
            result.score
        ));
    }
    
    if tavily_response.results.is_empty() {
        formatted_results.push_str("No results found for this query.");
    }
    
    Ok(formatted_results)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing for better debugging
    tracing_subscriber::fmt::init();
    dotenv::dotenv().ok();

    println!("🦀 Rust Deep Agents SDK - Simple Example");
    println!("=========================================");

    // Create tools using simple functions (mirrors Python: tools=[internet_search])
    // Just like Python, we define a regular function and the SDK handles the rest!
    let internet_search = create_tool(
        "internet_search",
        "Search the internet for information on any topic using Tavily API",
        |args: Value| async move {
            let query = args.get("query")
                .and_then(|v| v.as_str())
                .unwrap_or("default query");
            
            let max_results = args.get("max_results")
                .and_then(|v| v.as_u64())
                .map(|n| n as u32);
            
            // Call real Tavily search API
            match call_tavily_search(query, max_results).await {
                Ok(results) => Ok(results),
                Err(e) => {
                    // Fallback to a helpful error message
                    Ok(format!(
                        "❌ Search failed: {}. Please check your TAVILY_API_KEY environment variable.",
                        e
                    ))
                }
            }
        }
    );

    let tools = vec![internet_search];
    let instructions = "You are an expert researcher. Your job is to conduct thorough research, and then write a polished report.

You have access to tools that you can call to gather information. 

CRITICAL: When you need to use a tool, respond ONLY with JSON in this exact format (no other text):

```json
{
  \"tool_calls\": [
    {
      \"name\": \"tool_name\",
      \"args\": {
        \"parameter\": \"value\"
      }
    }
  ]
}
```

Available tools:

## `internet_search`
Use this to run an internet search for a given query. Parameters:
- query (string): The search query
- max_results (optional number): Maximum number of results to return

When you don't need to call a tool, respond normally with text.

IMPORTANT: For search requests, you MUST use the internet_search tool to get current information. Do not provide explanatory text with the JSON - respond with ONLY the JSON when calling tools.".to_string();

    // Create OpenAI model configuration
    let openai_config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")
            .map_err(|_| anyhow::anyhow!("OPENAI_API_KEY environment variable is required"))?,
        "gpt-4o-mini" // Using GPT-4o-mini (fast and cost-effective)
    );

    // Create a checkpointer for state persistence
    let checkpointer = Arc::new(InMemoryCheckpointer::new());

    // Create the agent using the builder pattern with OpenAI
    let agent = ConfigurableAgentBuilder::new(instructions)
        .with_openai_chat(openai_config)? // Use OpenAI instead of Anthropic
        .with_tools(tools)
        .with_checkpointer(checkpointer) // Enable state persistence
        .build()?;

    println!("✅ Deep Agent created successfully!");
    println!("📝 Agent description: {:?}", agent.describe().await);

    // Handle a message (mirrors Python: agent.invoke({"messages": [{"role": "user", "content": "..."}]}))
    let user_message = "Search the Web for infromation about Yafa cloud Services LLC pls";
    println!("\n🗣️  User: {}", user_message);

    let response = agent.handle_message(
        user_message,
        Arc::new(AgentStateSnapshot::default()),
    ).await?;

    println!("🤖 Agent: {}", response.content.as_text().unwrap_or("No response"));

    // Demonstrate state persistence
    println!("\n💾 Testing state persistence...");
    let thread_id = "example-thread".to_string();
    agent.save_state(&thread_id).await?;
    println!("✅ State saved for thread: {}", thread_id);

    let threads = agent.list_threads().await?;
    println!("📋 Available threads: {:?}", threads);

    Ok(())
}
