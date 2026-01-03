//! TOON Format Demo
//!
//! This example demonstrates how to use TOON (Token-Oriented Object Notation)
//! format for token-efficient system prompts.
//!
//! TOON provides 30-60% token reduction compared to JSON, which can significantly
//! reduce costs and improve response times when using LLM APIs.
//!
//! See: https://github.com/toon-format/toon
//!
//! ## What This Demo Shows
//!
//! 1. Creating an agent with TOON-formatted system prompts
//! 2. Comparing JSON vs TOON token usage
//! 3. Using ToonEncoder to encode tool results
//! 4. Real-world tool calling with TOON format

use agents_core::toon::ToonEncoder;
use agents_runtime::PromptFormat;
use agents_sdk::{
    persistence::InMemoryCheckpointer, tool, ConfigurableAgentBuilder, OpenAiChatModel,
    OpenAiConfig, SubAgentConfig,
};
use anyhow::Result;
use std::sync::Arc;
use tracing::info;

// A tool that returns structured data - perfect for TOON encoding
#[tool("Search for products and return results")]
fn search_products(query: String, limit: i32) -> String {
    info!(
        "🔍 SEARCH_PRODUCTS called: query='{}', limit={}",
        query, limit
    );

    // Simulate product search results
    let results = vec![
        serde_json::json!({
            "id": 1,
            "name": "Rust Programming Book",
            "price": 49.99,
            "in_stock": true
        }),
        serde_json::json!({
            "id": 2,
            "name": "Async Rust Course",
            "price": 99.99,
            "in_stock": true
        }),
        serde_json::json!({
            "id": 3,
            "name": "WebAssembly Guide",
            "price": 39.99,
            "in_stock": false
        }),
    ];

    // Encode results using TOON for efficient token usage
    let encoder = ToonEncoder::new();
    match encoder.encode(&serde_json::json!({ "products": results })) {
        Ok(toon_output) => {
            info!("📦 Returning TOON-encoded results:\n{}", toon_output);
            toon_output
        }
        Err(_) => {
            // Fallback to JSON if TOON encoding fails
            serde_json::to_string_pretty(&results).unwrap_or_default()
        }
    }
}

// A simple calculator tool
#[tool("Performs arithmetic calculations")]
fn calculate(expression: String) -> String {
    info!("🧮 CALCULATE called: expression='{}'", expression);

    // Simple evaluation (for demo purposes)
    if expression.contains("+") {
        let parts: Vec<&str> = expression.split('+').collect();
        if parts.len() == 2 {
            if let (Ok(a), Ok(b)) = (
                parts[0].trim().parse::<f64>(),
                parts[1].trim().parse::<f64>(),
            ) {
                return format!("{}", a + b);
            }
        }
    } else if expression.contains("*") {
        let parts: Vec<&str> = expression.split('*').collect();
        if parts.len() == 2 {
            if let (Ok(a), Ok(b)) = (
                parts[0].trim().parse::<f64>(),
                parts[1].trim().parse::<f64>(),
            ) {
                return format!("{}", a * b);
            }
        }
    }

    format!("Result: {}", expression)
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter("toon_format_demo=info,agents_runtime=warn")
        .init();

    // Load environment variables
    dotenvy::dotenv().ok();

    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set in .env file");

    info!("╔════════════════════════════════════════════════════════════╗");
    info!("║          TOON Format Demo - Token-Efficient Prompts        ║");
    info!("╠════════════════════════════════════════════════════════════╣");
    info!("║  TOON provides 30-60% token reduction vs JSON              ║");
    info!("║  https://github.com/toon-format/toon                       ║");
    info!("╚════════════════════════════════════════════════════════════╝");

    // Demo 1: Show TOON encoding
    demo_toon_encoding()?;

    // Demo 2: Create agent with TOON format
    demo_toon_agent(&api_key).await?;

    // Demo 3: Compare JSON vs TOON prompts
    demo_format_comparison();

    info!("\n✅ TOON Format Demo completed successfully!");
    Ok(())
}

/// Demonstrate ToonEncoder usage
fn demo_toon_encoding() -> Result<()> {
    info!("\n═══ Demo 1: ToonEncoder Usage ═══\n");

    let encoder = ToonEncoder::new();

    // Example 1: Simple object
    let simple_data = serde_json::json!({
        "name": "Alice",
        "role": "developer",
        "level": 5
    });

    info!("📝 Simple object:");
    info!("   JSON: {}", serde_json::to_string(&simple_data)?);
    info!("   TOON:\n{}", encoder.encode(&simple_data)?);

    // Example 2: Array of objects (where TOON really shines)
    let users = serde_json::json!({
        "users": [
            {"id": 1, "name": "Alice", "active": true},
            {"id": 2, "name": "Bob", "active": true},
            {"id": 3, "name": "Charlie", "active": false}
        ]
    });

    let json_str = serde_json::to_string(&users)?;
    let toon_str = encoder.encode(&users)?;

    info!("\n📊 Array of objects (best case for TOON):");
    info!("   JSON ({} chars):\n   {}", json_str.len(), json_str);
    info!("   TOON ({} chars):\n{}", toon_str.len(), toon_str);
    info!(
        "   📉 Size reduction: {:.1}%",
        (1.0 - toon_str.len() as f64 / json_str.len() as f64) * 100.0
    );

    // Example 3: Tool call format
    let tool_call = serde_json::json!({
        "tool": "search_products",
        "args": {
            "query": "rust books",
            "limit": 5
        }
    });

    info!("\n🔧 Tool call format:");
    info!("   JSON: {}", serde_json::to_string(&tool_call)?);
    info!("   TOON:\n{}", encoder.encode(&tool_call)?);

    Ok(())
}

/// Create and test an agent using TOON format
async fn demo_toon_agent(api_key: &str) -> Result<()> {
    info!("\n═══ Demo 2: Agent with TOON Format ═══\n");

    // Create OpenAI configuration
    let openai_config = OpenAiConfig::new(api_key.to_string(), "gpt-4o-mini");

    // Create a sub-agent for analysis
    let analyzer_subagent = SubAgentConfig::new(
        "analyzer",
        "Data analysis specialist",
        "You analyze data and provide insights. Be concise and data-driven.",
    );

    // Build the agent with TOON format
    info!("🤖 Creating agent with PromptFormat::Toon...");
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful shopping assistant. Search for products and help users find what they need."
    )
    .with_model(Arc::new(OpenAiChatModel::new(openai_config)?))
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .with_prompt_format(PromptFormat::Toon)  // <-- Use TOON format!
    .with_tool(SearchProductsTool::as_tool())
    .with_tool(CalculateTool::as_tool())
    .with_subagent_config(vec![analyzer_subagent])
    .build()?;

    info!("✅ Agent created with TOON-formatted system prompt");
    info!("   This reduces token usage by 30-60% for tool examples");

    // Test the agent with a product search
    info!("\n📨 Sending test message: \"Search for Rust programming books\"");

    let response = agent
        .handle_message(
            "Search for Rust programming books and tell me the best option",
            Arc::new(agents_sdk::state::AgentStateSnapshot::default()),
        )
        .await?;

    let response_text = response.content.as_text().unwrap_or("No response");
    info!("\n📬 Agent response:\n{}", response_text);

    Ok(())
}

/// Compare JSON vs TOON system prompts
fn demo_format_comparison() {
    use agents_runtime::prompts::{
        get_deep_agent_system_prompt, get_deep_agent_system_prompt_toon,
    };

    info!("\n═══ Demo 3: Format Comparison ═══\n");

    let custom_instructions = "You are a helpful assistant.";

    let json_prompt = get_deep_agent_system_prompt(custom_instructions);
    let toon_prompt = get_deep_agent_system_prompt_toon(custom_instructions);

    info!("📊 System Prompt Comparison:");
    info!("   JSON format: {} characters", json_prompt.len());
    info!("   TOON format: {} characters", toon_prompt.len());
    info!(
        "   📉 Character reduction: {:.1}%",
        (1.0 - toon_prompt.len() as f64 / json_prompt.len() as f64) * 100.0
    );

    // Estimate token savings (rough approximation: ~4 chars per token)
    let json_tokens_est = json_prompt.len() / 4;
    let toon_tokens_est = toon_prompt.len() / 4;

    info!("\n💰 Estimated Token Savings:");
    info!("   JSON: ~{} tokens", json_tokens_est);
    info!("   TOON: ~{} tokens", toon_tokens_est);
    info!(
        "   Saved: ~{} tokens per request",
        json_tokens_est - toon_tokens_est
    );

    // Calculate cost savings (using GPT-4o-mini pricing: $0.15/1M input tokens)
    let cost_per_million = 0.15;
    let savings_per_request =
        (json_tokens_est - toon_tokens_est) as f64 * cost_per_million / 1_000_000.0;
    let requests_per_million_tokens = 1_000_000.0 / json_tokens_est as f64;
    let savings_per_million = savings_per_request * requests_per_million_tokens;

    info!("\n💵 Cost Savings (GPT-4o-mini pricing):");
    info!("   Per request: ${:.6}", savings_per_request);
    info!(
        "   Per 1M tokens worth of requests: ${:.2}",
        savings_per_million
    );

    info!("\n📝 Sample TOON tool call format:");
    info!("```toon");
    info!("tool_calls[1]:");
    info!("  name: search_products");
    info!("  args:");
    info!("    query: rust books");
    info!("    limit: 5");
    info!("```");
}
