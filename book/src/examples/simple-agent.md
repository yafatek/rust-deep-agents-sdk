# Simple Agent Example

The most basic agent setup with OpenAI.

## Code

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load API key
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    // Create model
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);
    
    // Build agent
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant. Be concise and friendly."
    )
    .with_model(model)
    .build()?;
    
    // Send message
    let response = agent.handle_message(
        "What is Rust programming language?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or_default());
    
    Ok(())
}
```

## Run It

```bash
cd examples/simple-agent
export OPENAI_API_KEY="your-key"
cargo run
```

## What It Demonstrates

- Basic `ConfigurableAgentBuilder` usage
- OpenAI model configuration
- Single message handling
- Response extraction

## Next Steps

- Add [tools](./tool-creation.md) for agent capabilities
- Enable [persistence](../persistence/overview.md) for conversations
- Track [token usage](../features/token-tracking.md) for costs

