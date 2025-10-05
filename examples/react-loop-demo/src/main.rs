/// Deep Agents Demo - Multi-Agent Delegation with ReAct Loop
///
/// This demonstrates the Deep Agents pattern:
/// - Main orchestrator delegates to specialized sub-agents
/// - Each sub-agent has its own tools and expertise  
/// - Sub-agents use ReAct loop to complete tasks
use agents_sdk::{tool, ConfigurableAgentBuilder, OpenAiConfig, SubAgentConfig};
use std::sync::Arc;

#[tool("Search for automotive services by keyword")]
fn search_services(query: String) -> String {
    println!("🔍 [CATALOG-AGENT] search_services('{}')", query);
    let result = "Found services:\n- Battery Replacement: 350 AED\n- Oil Change: 150 AED";
    println!("   ↳ Found 2 services");
    result.to_string()
}

#[tool("Generate a price quote for a service on a specific vehicle type")]
fn generate_quote(service_name: String, vehicle_type: String) -> String {
    println!(
        "💰 [QUOTE-AGENT] generate_quote('{}', '{}')",
        service_name, vehicle_type
    );

    let base_price = if service_name.contains("Battery") {
        350.0
    } else {
        150.0
    };
    let multiplier = match vehicle_type.to_lowercase().as_str() {
        "suv" => 1.2,
        "luxury" => 1.5,
        _ => 1.0,
    };
    let final_price = base_price * multiplier;

    let result = format!(
        "Quote: {} on {} = {} AED",
        service_name, vehicle_type, final_price
    );
    println!("   ↳ {}", result);
    result
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("react_loop_demo=info,agents_runtime=warn")
        .init();

    println!("\n🚀 Deep Agents Demo - Multi-Agent Delegation\n");

    dotenvy::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");
    let openai_config = OpenAiConfig::new(api_key, "gpt-4o-mini");

    // Sub-agent 1: Catalog specialist
    let catalog_agent = SubAgentConfig::new(
        "catalog-agent",
        "Service catalog specialist",
        "You search for services. Use search_services tool and return the results.",
    )
    .with_tools(vec![SearchServicesTool::as_tool()]);

    // Sub-agent 2: Quote specialist
    let quote_agent = SubAgentConfig::new(
        "quote-agent",
        "Quote generation specialist",
        "You generate quotes. Use generate_quote tool with service name and vehicle type.",
    )
    .with_tools(vec![GenerateQuoteTool::as_tool()]);

    // Main orchestrator agent
    let main_agent = ConfigurableAgentBuilder::new(
        "You coordinate automotive services. \
         For quotes:\n\
         1. Delegate to 'catalog-agent' to search services\n\
         2. Delegate to 'quote-agent' to generate quote\n\
         3. Present the result\n\
         Use task() to delegate.",
    )
    .with_openai_chat(openai_config)?
    .with_subagent_config(vec![catalog_agent, quote_agent])
    .build()?;

    let query = "I need a quote for battery replacement on my SUV";

    println!("📝 Customer: {}\n", query);
    println!("Expected flow:");
    println!("  Main → catalog-agent → search_services → results");
    println!("  Main → quote-agent → generate_quote → quote");
    println!("  Main → customer response\n");
    println!("---\n");

    let response = main_agent
        .handle_message(
            query,
            Arc::new(agents_sdk::state::AgentStateSnapshot::default()),
        )
        .await?;

    println!("\n---\n");
    println!(
        "✅ Response: {}\n",
        response.content.as_text().unwrap_or("No text")
    );
    println!("✨ Deep Agents pattern demonstrated successfully!");

    Ok(())
}
