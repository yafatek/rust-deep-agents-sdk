use agents_core::persistence::InMemoryCheckpointer;
use agents_core::state::AgentStateSnapshot;
use agents_macros::tool;
use agents_sdk::{ConfigurableAgentBuilder, GeminiChatModel, GeminiConfig};
use std::sync::Arc;

#[tool("Divides first number by second number")]
pub fn divide(a: f64, b: f64) -> String {
    tracing::warn!("➗ GEMINI TOOL CALLED: divide({}, {})", a, b);
    if b == 0.0 {
        "Error: Cannot divide by zero".to_string()
    } else {
        let result = a / b;
        tracing::warn!("✅ Result: {} / {} = {}", a, b, result);
        format!("{}", result)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("warn").init();

    dotenv::dotenv().ok();

    let api_key =
        std::env::var("GEMINI_API_KEY").map_err(|_| anyhow::anyhow!("GEMINI_API_KEY not found"))?;

    println!("🤖 Testing Gemini Tool Support");
    println!("==============================\n");

    let tools = vec![DivideTool::as_tool()];
    let config = GeminiConfig {
        api_key,
        model: "gemini-pro".to_string(),
        api_url: None,
    };

    let agent = ConfigurableAgentBuilder::new(
        r#"You are a math assistant. When asked to calculate division, use the divide tool.
        
When you need to use a tool, respond with JSON:
```json
{
  "tool_calls": [
    {
      "name": "divide",
      "args": {
        "a": 10,
        "b": 2
      }
    }
  ]
}
```"#,
    )
    .with_model(Arc::new(GeminiChatModel::new(config)?))
    .with_tools(tools)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .build()?;

    let test = "What is 100 divided by 4?";
    println!("👤 User: {}", test);

    let response = agent
        .handle_message(test, Arc::new(AgentStateSnapshot::default()))
        .await?;

    println!(
        "🤖 Agent: {}\n",
        response.content.as_text().unwrap_or("No response")
    );

    println!("✅ If you see '➗ GEMINI TOOL CALLED' above, it works!");

    Ok(())
}
