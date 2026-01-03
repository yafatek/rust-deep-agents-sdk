# Google Gemini

Configure and use Google Gemini models with the Deep Agents SDK.

## Quick Start

```rust
use agents_sdk::{GeminiConfig, GeminiChatModel, ConfigurableAgentBuilder};
use std::sync::Arc;

let api_key = std::env::var("GOOGLE_API_KEY")?;
let config = GeminiConfig::new(api_key, "gemini-2.5-pro");
let model = Arc::new(GeminiChatModel::new(config)?);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .build()?;
```

## Configuration

### Basic Configuration

```rust
let config = GeminiConfig::new(api_key, model_name);
```

### Available Models

| Model | Best For | Context | Characteristics |
|-------|----------|---------|-----------------|
| `gemini-2.5-pro` | Complex tasks | 1M tokens | Most capable |
| `gemini-2.5-flash` | Fast responses | 1M tokens | Very fast |
| `gemini-2.0-flash` | Quick tasks | 1M tokens | Fastest |

> **Note**: The SDK is model-agnostic. Any Gemini model string will work.

### Advanced Configuration

```rust
let config = GeminiConfig::new(api_key, "gemini-2.5-pro")
    .with_timeout(Duration::from_secs(120));
```

## Environment Variables

```bash
export GOOGLE_API_KEY="..."
```

## Gemini's Strengths

### Massive Context Window

Gemini supports up to 1 million tokens:

```rust
let agent = ConfigurableAgentBuilder::new(
    "You are a document analyst with access to extensive documents."
)
.with_model(Arc::new(GeminiChatModel::new(
    GeminiConfig::new(api_key, "gemini-2.5-pro")
)?))
.build()?;

// Process very long documents
let response = agent.handle_message(
    &format!("Summarize this book:\n\n{}", entire_book_text),
    state
).await?;
```

### Speed with Flash Models

```rust
// Ultra-fast responses
let config = GeminiConfig::new(api_key, "gemini-2.5-flash");
```

## Tool Calling

Gemini supports function calling:

```rust
use agents_sdk::tool;

#[tool("Get weather information")]
async fn get_weather(city: String) -> String {
    format!("Weather in {}: Sunny, 25°C", city)
}

let agent = ConfigurableAgentBuilder::new("You can check weather for any city.")
    .with_model(model)
    .with_tool(GetWeatherTool::as_tool())
    .build()?;
```

## Token Costs

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

// Gemini costs (example)
let costs = TokenCosts {
    input_cost_per_million: 1.25,
    output_cost_per_million: 5.00,
};

let config = TokenTrackingConfig {
    enabled: true,
    custom_costs: Some(costs),
    ..Default::default()
};
```

## Multi-Modal Support

Gemini supports vision capabilities:

```rust
// Note: Multi-modal support coming in future SDK versions
// For now, use text-based interactions
```

## Error Handling

```rust
match GeminiChatModel::new(config) {
    Ok(model) => { /* use model */ }
    Err(e) => {
        let error_str = e.to_string();
        if error_str.contains("invalid") {
            eprintln!("Invalid API key");
        } else if error_str.contains("quota") {
            eprintln!("Quota exceeded");
        } else {
            eprintln!("Gemini error: {}", e);
        }
    }
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    GeminiConfig,
    GeminiChatModel,
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tool("Search the knowledge base")]
async fn search_kb(query: String) -> String {
    format!("Found information about: {}", query)
}

#[tool("Calculate expression")]
fn calculate(expression: String) -> String {
    // Simple calculator
    format!("Result: {}", expression)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("GOOGLE_API_KEY")?;
    
    let config = GeminiConfig::new(api_key, "gemini-2.5-flash");
    let model = Arc::new(GeminiChatModel::new(config)?);
    
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful assistant powered by Gemini. 
         You can search the knowledge base and perform calculations."
    )
    .with_model(model)
    .with_tools(vec![
        SearchKbTool::as_tool(),
        CalculateTool::as_tool(),
    ])
    .build()?;
    
    let response = agent.handle_message(
        "What is 2 + 2?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or_default());
    
    Ok(())
}
```

## Best Practices

### 1. Use Flash for Speed

```rust
// For quick, interactive applications
let config = GeminiConfig::new(api_key, "gemini-2.5-flash");
```

### 2. Use Pro for Quality

```rust
// For complex reasoning and analysis
let config = GeminiConfig::new(api_key, "gemini-2.5-pro");
```

### 3. Leverage Long Context

```rust
// Gemini excels at processing long documents
let agent = ConfigurableAgentBuilder::new(
    "Analyze and compare all the documents provided."
)
.with_model(model)
.build()?;
```

### 4. Handle Quotas

```rust
use tokio::time::sleep;
use std::time::Duration;

async fn with_quota_retry<T, F, Fut>(f: F) -> anyhow::Result<T>
where
    F: Fn() -> Fut,
    Fut: std::future::Future<Output = anyhow::Result<T>>,
{
    let mut attempts = 0;
    loop {
        match f().await {
            Ok(result) => return Ok(result),
            Err(e) if attempts < 3 && e.to_string().contains("quota") => {
                attempts += 1;
                sleep(Duration::from_secs(60)).await;
            }
            Err(e) => return Err(e),
        }
    }
}
```

