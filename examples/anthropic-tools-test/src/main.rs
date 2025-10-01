use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_macros::tool;
use agents_sdk::{AnthropicConfig, ConfigurableAgentBuilder};
use std::sync::Arc;

#[tool("Multiplies two numbers together")]
pub fn multiply(a: i32, b: i32) -> i32 {
    tracing::warn!("🔢 ANTHROPIC TOOL CALLED: multiply({}, {})", a, b);
    let result = a * b;
    tracing::warn!("✅ Result: {} * {} = {}", a, b, result);
    result
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter("warn")
        .init();

    dotenv::dotenv().ok();

    let api_key = std::env::var("ANTHROPIC_API_KEY")
        .map_err(|_| anyhow::anyhow!("ANTHROPIC_API_KEY not found"))?;

    println!("🤖 Testing Anthropic Tool Support");
    println!("================================\n");

    let tools = vec![MultiplyTool::as_tool()];
    let config = AnthropicConfig {
        api_key,
        model: "claude-3-5-sonnet-20240620".to_string(),
        max_output_tokens: 1024,
        api_url: None,
        api_version: Some("2023-06-01".to_string()),
    };

    let agent = ConfigurableAgentBuilder::new(
        r#"You are a math assistant. When asked to calculate, use the multiply tool.
        
When you need to use a tool, respond with JSON:
```json
{
  "tool_calls": [
    {
      "name": "multiply",
      "args": {
        "a": 5,
        "b": 3
      }
    }
  ]
}
```"#,
    )
    .with_model(Arc::new(agents_sdk::AnthropicMessagesModel::new(config)?))
    .with_tools(tools)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .build()?;

    let test = "What is 7 times 8?";
    println!("👤 User: {}", test);

    let response = agent
        .handle_message(test, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!(
        "🤖 Agent: {}\n",
        response.content.as_text().unwrap_or("No response")
    );

    println!("✅ If you see '🔢 ANTHROPIC TOOL CALLED' above, it works!");

    Ok(())
}

