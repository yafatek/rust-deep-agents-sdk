# OpenAI

Configure and use OpenAI models with the Deep Agents SDK.

## Quick Start

```rust
use agents_sdk::{OpenAiConfig, OpenAiChatModel, ConfigurableAgentBuilder};
use std::sync::Arc;

let api_key = std::env::var("OPENAI_API_KEY")?;
let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
let model = Arc::new(OpenAiChatModel::new(config)?);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .build()?;
```

## Configuration

### Basic Configuration

```rust
let config = OpenAiConfig::new(api_key, model_name);
```

### Available Models

| Model | Best For | Context | Cost |
|-------|----------|---------|------|
| `gpt-5.2` | Latest capabilities | 128K | $$$ |
| `gpt-4o` | Best quality | 128K | $$ |
| `gpt-4o-mini` | Fast & cheap | 128K | $ |
| `o1-pro` | Complex reasoning | 128K | $$$$ |
| `o1-mini` | Fast reasoning | 128K | $$ |

> **Note**: The SDK is model-agnostic. Any model string supported by OpenAI will work.

### Advanced Configuration

```rust
let config = OpenAiConfig::new(api_key, "gpt-4o")
    .with_base_url("https://custom-endpoint.com")  // Custom endpoint
    .with_organization("org-xxx")                   // Organization ID
    .with_timeout(Duration::from_secs(120));        // Request timeout
```

## Environment Variables

```bash
export OPENAI_API_KEY="sk-..."
export OPENAI_ORG_ID="org-..."           # Optional
export OPENAI_BASE_URL="https://..."     # Optional
```

## Token Costs

Configure accurate cost tracking:

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

let costs = TokenCosts {
    input_cost_per_million: 0.15,   // $0.15 per 1M input tokens
    output_cost_per_million: 0.60,  // $0.60 per 1M output tokens
};

let config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    custom_costs: Some(costs),
    ..Default::default()
};
```

### Preset Cost Configurations

```rust
// GPT-4o-mini costs
TokenCosts::openai_gpt4o_mini()

// GPT-4o costs
TokenCosts::openai_gpt4o()
```

## Streaming

OpenAI supports streaming responses:

```rust
use agents_sdk::llm::{ChunkStream, StreamChunk};

// Streaming is automatic in handle_message
// Subscribe to events to receive streaming tokens
let dispatcher = Arc::new(EventDispatcher::new());
let mut receiver = dispatcher.subscribe();

let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_event_dispatcher(dispatcher)
    .build()?;

// Listen for streaming tokens
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        if let AgentEvent::StreamingToken(token) = event {
            print!("{}", token.content);
        }
    }
});
```

## Tool Calling

OpenAI's function calling works seamlessly:

```rust
use agents_sdk::tool;

#[tool("Search the web")]
async fn search(query: String) -> String {
    // Implementation
}

let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_tool(SearchTool::as_tool())
    .build()?;
```

## Error Handling

```rust
match OpenAiChatModel::new(config) {
    Ok(model) => {
        // Use model
    }
    Err(e) => {
        if e.to_string().contains("invalid_api_key") {
            eprintln!("Invalid API key. Check OPENAI_API_KEY.");
        } else if e.to_string().contains("rate_limit") {
            eprintln!("Rate limited. Try again later.");
        } else {
            eprintln!("OpenAI error: {}", e);
        }
    }
}
```

## Best Practices

### 1. Choose the Right Model

```rust
// For simple tasks, use gpt-4o-mini (fast, cheap)
let config = OpenAiConfig::new(api_key, "gpt-4o-mini");

// For complex tasks, use gpt-4o (better quality)
let config = OpenAiConfig::new(api_key, "gpt-4o");

// For reasoning tasks, use o1 models
let config = OpenAiConfig::new(api_key, "o1-mini");
```

### 2. Set Reasonable Timeouts

```rust
let config = OpenAiConfig::new(api_key, "gpt-4o")
    .with_timeout(Duration::from_secs(60));
```

### 3. Handle Rate Limits

```rust
use tokio::time::sleep;
use std::time::Duration;

async fn with_retry<T, F, Fut>(f: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < 3 && e.to_string().contains("rate_limit") => {
                attempts += 1;
                sleep(Duration::from_secs(2u64.pow(attempts))).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    tool,
    TokenTrackingConfig,
    TokenCosts,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tool("Get the current time")]
fn get_time() -> String {
    chrono::Utc::now().to_rfc3339()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);
    
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_tool(GetTimeTool::as_tool())
        .with_token_tracking_config(TokenTrackingConfig {
            enabled: true,
            emit_events: true,
            custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
            ..Default::default()
        })
        .build()?;
    
    let response = agent.handle_message(
        "What time is it?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or_default());
    
    if let Some(usage) = &response.usage {
        println!("Tokens used: {} input, {} output", 
            usage.prompt_tokens, usage.completion_tokens);
    }
    
    Ok(())
}
```

