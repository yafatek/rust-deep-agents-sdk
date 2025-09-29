use agents_sdk::{
    get_default_model, state::AgentStateSnapshot, tool, ToolParameterSchema, ToolResult,
    ConfigurableAgentBuilder,
};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing Unified Agents SDK");
    dotenv::dotenv().ok();

    // Create parameter schema for the greet tool
    let mut params = HashMap::new();
    params.insert(
        "name".to_string(),
        ToolParameterSchema::string("The name of the person to greet"),
    );
    let param_schema = ToolParameterSchema::object(
        "Greet tool parameters",
        params,
        vec!["name".to_string()],
    );

    // Create a simple tool using the unified SDK
    let greet_tool = tool(
        "greet",
        "Greets a person by name",
        param_schema,
        |args: Value, ctx| async move {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("World");
            Ok(ToolResult::text(&ctx, format!("Hello, {}! 👋", name)))
        },
    );

    // Build an agent using the unified SDK
    println!("🔧 Building agent with unified SDK...");
    let agent = ConfigurableAgentBuilder::new("You are a friendly assistant that greets people.")
        .with_model(get_default_model()?)
        .with_tool(greet_tool)
        .build()?;

    // Test the agent
    println!("🤖 Testing agent...");
    let response = agent
        .handle_message(
            "Please greet Alice using the greet tool",
            Arc::new(AgentStateSnapshot::default()),
        )
        .await?;

    println!("✅ Agent Response: {:?}", response);
    println!("🎉 Unified SDK test completed successfully!");

    Ok(())
}
