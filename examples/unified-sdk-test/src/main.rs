use agents_sdk::{
    create_tool, get_default_model, state::AgentStateSnapshot, ConfigurableAgentBuilder,
};
use serde_json::Value;
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing Unified Agents SDK");

    // Create a simple tool using the unified SDK
    let greet_tool = create_tool(
        "greet",
        "Greets a person by name",
        |args: Value| async move {
            let name = args.get("name").and_then(|v| v.as_str()).unwrap_or("World");
            Ok(format!("Hello, {}! 👋", name))
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
