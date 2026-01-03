# Anthropic Claude

Configure and use Anthropic Claude models with the Deep Agents SDK.

## Quick Start

```rust
use agents_sdk::{AnthropicConfig, AnthropicMessagesModel, ConfigurableAgentBuilder};
use std::sync::Arc;

let api_key = std::env::var("ANTHROPIC_API_KEY")?;
let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096);
let model = Arc::new(AnthropicMessagesModel::new(config)?);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .build()?;
```

## Configuration

### Basic Configuration

```rust
let config = AnthropicConfig::new(
    api_key,        // Your API key
    model_name,     // Model identifier
    max_tokens,     // Maximum response tokens
);
```

### Available Models

| Model | Best For | Context | Characteristics |
|-------|----------|---------|-----------------|
| `claude-opus-4.5` | Complex tasks | 200K | Most capable |
| `claude-sonnet-4.5` | Balanced | 200K | Fast & capable |
| `claude-haiku-4.5` | Quick tasks | 200K | Fastest |

> **Note**: The SDK is model-agnostic. Any Claude model string will work.

### Advanced Configuration

```rust
let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)
    .with_base_url("https://custom-endpoint.com")
    .with_anthropic_version("2024-01-01")
    .with_timeout(Duration::from_secs(120));
```

### Custom Headers

For specific features or beta access:

```rust
let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)
    .with_header("anthropic-beta", "prompt-caching-2024-07-31");
```

## Environment Variables

```bash
export ANTHROPIC_API_KEY="sk-ant-..."
```

## Max Tokens

Unlike OpenAI, Anthropic requires specifying `max_tokens`:

```rust
// For longer responses
let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 8192);

// For shorter responses (saves costs)
let config = AnthropicConfig::new(api_key, "claude-haiku-4.5", 1024);
```

## Token Costs

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

// Claude Sonnet costs (example)
let costs = TokenCosts {
    input_cost_per_million: 3.0,    // $3 per 1M input tokens
    output_cost_per_million: 15.0,  // $15 per 1M output tokens
};

let config = TokenTrackingConfig {
    enabled: true,
    custom_costs: Some(costs),
    ..Default::default()
};
```

## Tool Calling

Anthropic's tool use works seamlessly:

```rust
use agents_sdk::tool;

#[tool("Calculate compound interest")]
fn calculate_interest(principal: f64, rate: f64, years: u32) -> String {
    let amount = principal * (1.0 + rate / 100.0).powi(years as i32);
    format!("${:.2}", amount)
}

let agent = ConfigurableAgentBuilder::new("You are a financial assistant.")
    .with_model(model)
    .with_tool(CalculateInterestTool::as_tool())
    .build()?;
```

## Prompt Caching

Enable prompt caching for repeated system prompts:

```rust
let agent = ConfigurableAgentBuilder::new("Your detailed system prompt...")
    .with_model(model)
    .with_prompt_caching(true)  // Enable caching
    .build()?;
```

## Claude's Strengths

### Code Generation

Claude excels at code tasks:

```rust
let agent = ConfigurableAgentBuilder::new(
    "You are an expert Rust programmer. Write clean, idiomatic code."
)
.with_model(Arc::new(AnthropicMessagesModel::new(
    AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)
)?))
.build()?;
```

### Long Context

Claude handles long documents well:

```rust
// Claude supports 200K token context
let agent = ConfigurableAgentBuilder::new(
    "You are a document analyst. Carefully read and analyze the provided documents."
)
.with_model(model)
.build()?;

// Send long document
let response = agent.handle_message(
    &format!("Analyze this document:\n\n{}", long_document),
    state
).await?;
```

## Error Handling

```rust
match AnthropicMessagesModel::new(config) {
    Ok(model) => { /* use model */ }
    Err(e) => {
        let error_str = e.to_string();
        if error_str.contains("authentication") {
            eprintln!("Invalid API key");
        } else if error_str.contains("rate_limit") {
            eprintln!("Rate limited");
        } else if error_str.contains("overloaded") {
            eprintln!("API overloaded, retry later");
        } else {
            eprintln!("Anthropic error: {}", e);
        }
    }
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    AnthropicConfig,
    AnthropicMessagesModel,
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tool("Search for information")]
async fn search(query: String) -> String {
    format!("Results for: {}", query)
}

#[tool("Read file contents")]
async fn read_file(path: String) -> String {
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| format!("Error: {}", e))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("ANTHROPIC_API_KEY")?;
    
    let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096);
    let model = Arc::new(AnthropicMessagesModel::new(config)?);
    
    let agent = ConfigurableAgentBuilder::new(
        "You are Claude, an AI assistant created by Anthropic. 
         You can search for information and read files."
    )
    .with_model(model)
    .with_tools(vec![
        SearchTool::as_tool(),
        ReadFileTool::as_tool(),
    ])
    .with_prompt_caching(true)
    .build()?;
    
    let response = agent.handle_message(
        "Search for Rust async programming best practices",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or_default());
    
    Ok(())
}
```

## Best Practices

### 1. Choose Appropriate Max Tokens

```rust
// Match max_tokens to expected response length
// Short answers
AnthropicConfig::new(api_key, "claude-haiku-4.5", 256)

// Medium responses  
AnthropicConfig::new(api_key, "claude-sonnet-4.5", 2048)

// Long-form content
AnthropicConfig::new(api_key, "claude-sonnet-4.5", 8192)
```

### 2. Use Haiku for Simple Tasks

```rust
// Fast, cheap for simple queries
let haiku = AnthropicConfig::new(api_key, "claude-haiku-4.5", 1024);

// Sonnet for more complex tasks
let sonnet = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096);
```

### 3. Leverage Long Context

```rust
// Claude handles large documents well
let agent = ConfigurableAgentBuilder::new(
    "Analyze the entire codebase and provide insights."
)
.with_model(model)
.build()?;
```

