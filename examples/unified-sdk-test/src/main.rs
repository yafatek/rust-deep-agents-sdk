use agents_sdk::{
    get_default_model, state::AgentStateSnapshot, tool, ConfigurableAgentBuilder,
};
use std::sync::Arc;

// Define a tool using the #[tool] macro - clean and simple!
#[tool("Greets a person by name")]
fn greet(name: String) -> String {
    format!("Hello, {}! 👋", name)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    println!("🧪 Testing Unified Agents SDK with #[tool] macro");
    dotenv::dotenv().ok();

    // Build an agent using the macro-generated tool
    println!("🔧 Building agent with unified SDK...");
    let agent = ConfigurableAgentBuilder::new("You are a friendly assistant that greets people.")
        .with_model(get_default_model()?)
        .with_tool(GreetTool::as_tool())
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
    println!("🎉 Unified SDK test with #[tool] macro completed successfully!");

    Ok(())
}
