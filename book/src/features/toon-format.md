# TOON Format

Token-Oriented Object Notation for 30-60% token reduction.

## Overview

[TOON](https://github.com/toon-format/toon) is a compact, human-readable data format designed specifically for LLM prompts. It reduces token usage by:

- **Omitting redundant keys** in tabular data
- **Using indentation** instead of brackets
- **Folding nested structures** efficiently

## Quick Start

### Enable the Feature

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["toon"] }
```

### Use TOON in Your Agent

```rust
use agents_sdk::{ConfigurableAgentBuilder, PromptFormat};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_prompt_format(PromptFormat::Toon)  // Enable TOON
    .build()?;
```

## JSON vs TOON Comparison

### Simple Object

**JSON (23 tokens):**
```json
{"name": "Alice", "age": 30, "active": true}
```

**TOON (15 tokens):**
```toon
name: Alice
age: 30
active: true
```

### Tabular Array

**JSON (89 tokens):**
```json
{
  "users": [
    {"id": 1, "name": "Alice", "role": "admin"},
    {"id": 2, "name": "Bob", "role": "user"},
    {"id": 3, "name": "Charlie", "role": "user"}
  ]
}
```

**TOON (35 tokens):**
```toon
users[3]{id,name,role}:
  1,Alice,admin
  2,Bob,user
  3,Charlie,user
```

### Tool Call Example

**JSON:**
```json
{
  "tool_calls": [
    {
      "name": "search",
      "args": {"query": "Rust programming", "limit": 5}
    }
  ]
}
```

**TOON:**
```toon
tool_calls[1]{name,args}:
  search,
    query: Rust programming
    limit: 5
```

## ToonEncoder API

### Basic Usage

```rust
use agents_core::toon::ToonEncoder;
use serde_json::json;

let encoder = ToonEncoder::new();

let data = json!({
    "products": [
        {"id": 1, "name": "Widget", "price": 9.99},
        {"id": 2, "name": "Gadget", "price": 19.99}
    ]
});

let toon = encoder.encode(&data)?;
// products[2]{id,name,price}:
//   1,Widget,9.99
//   2,Gadget,19.99
```

### Configuration Options

```rust
use agents_core::toon::ToonEncoder;

// Default encoder
let encoder = ToonEncoder::new();

// With custom indentation
let encoder = ToonEncoder::new()
    .with_spaces(4);  // 4 spaces

// With tabs
let encoder = ToonEncoder::new()
    .with_tabs();

// Compact encoder
let encoder = ToonEncoder::compact();
```

### Static Method

```rust
// Encode with defaults
let toon = ToonEncoder::encode_default(&my_data)?;
```

## PromptFormat Enum

```rust
use agents_sdk::PromptFormat;

// JSON format (default) - Most compatible
PromptFormat::Json

// TOON format - Token efficient
PromptFormat::Toon
```

## Using with DeepAgentConfig

```rust
use agents_runtime::agent::config::DeepAgentConfig;
use agents_runtime::PromptFormat;

let config = DeepAgentConfig::new("instructions", planner)
    .with_prompt_format(PromptFormat::Toon);
```

## Token Savings

### By Data Type

| Data Type | JSON | TOON | Savings |
|-----------|------|------|---------|
| Simple object | 20 | 15 | 25% |
| Array of objects | 100 | 45 | 55% |
| Nested object | 50 | 35 | 30% |
| Tool call | 30 | 15 | 50% |

### System Prompt Impact

| Component | JSON | TOON | Savings |
|-----------|------|------|---------|
| Tool examples | ~500 | ~250 | 50% |
| Full prompt | ~1500 | ~1100 | 27% |

### Cost Savings (GPT-4o-mini)

| Requests | JSON Cost | TOON Cost | Savings |
|----------|-----------|-----------|---------|
| 1,000 | $0.23 | $0.17 | $0.06 |
| 100,000 | $22.50 | $16.50 | $6.00 |
| 1M | $225 | $165 | $60 |

## When to Use TOON

### Recommended

- **High-volume applications** - Cost savings compound
- **Structured tool results** - Arrays compress well
- **Token-constrained contexts** - More room for content
- **Production deployments** - Lower costs at scale

### Consider JSON Instead

- **Debugging** - JSON is more widely supported
- **Compatibility-critical** - Some edge cases
- **Simple data** - Single values don't benefit
- **External integrations** - When systems expect JSON

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    PromptFormat,
    tool,
    state::AgentStateSnapshot,
};
use agents_core::toon::ToonEncoder;
use serde_json::json;
use std::sync::Arc;

#[tool("Search for products")]
async fn search_products(query: String, limit: u32) -> String {
    let results = vec![
        json!({"id": 1, "name": "Laptop", "price": 999.99}),
        json!({"id": 2, "name": "Mouse", "price": 29.99}),
        json!({"id": 3, "name": "Keyboard", "price": 79.99}),
    ];
    
    // Return as TOON for efficiency
    let encoder = ToonEncoder::new();
    let limited: Vec<_> = results.into_iter().take(limit as usize).collect();
    encoder.encode(&json!({"results": limited}))
        .unwrap_or_else(|_| serde_json::to_string(&limited).unwrap())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    // Compare JSON vs TOON
    println!("=== Encoding Comparison ===\n");
    
    let data = json!({
        "users": [
            {"id": 1, "name": "Alice", "role": "admin"},
            {"id": 2, "name": "Bob", "role": "user"}
        ]
    });
    
    let json_str = serde_json::to_string(&data)?;
    let toon_str = ToonEncoder::encode_default(&data)?;
    
    println!("JSON ({} chars):\n{}\n", json_str.len(), json_str);
    println!("TOON ({} chars):\n{}\n", toon_str.len(), toon_str);
    println!("Savings: {:.1}%\n", 
        (1.0 - toon_str.len() as f64 / json_str.len() as f64) * 100.0);

    // Create agent with TOON format
    let agent = ConfigurableAgentBuilder::new(
        "You are a shopping assistant. Search for products efficiently."
    )
    .with_model(model)
    .with_tool(SearchProductsTool::as_tool())
    .with_prompt_format(PromptFormat::Toon)  // Enable TOON
    .with_token_tracking(true)
    .build()?;

    let response = agent.handle_message(
        "Find me 2 products under $100",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("Response: {}", response.content.as_text().unwrap_or_default());
    
    if let Some(usage) = &response.usage {
        println!("Tokens used: {}", usage.total_tokens);
    }

    Ok(())
}
```

## Best Practices

### 1. Enable for Production

```rust
// Development: JSON for debugging
#[cfg(debug_assertions)]
let format = PromptFormat::Json;

// Production: TOON for efficiency
#[cfg(not(debug_assertions))]
let format = PromptFormat::Toon;

.with_prompt_format(format)
```

### 2. Track Savings

```rust
// Compare before/after enabling TOON
let json_tokens = measure_tokens(PromptFormat::Json);
let toon_tokens = measure_tokens(PromptFormat::Toon);
let savings = (1.0 - toon_tokens as f64 / json_tokens as f64) * 100.0;
tracing::info!("TOON savings: {:.1}%", savings);
```

### 3. Use for Tool Results

```rust
#[tool("Get data")]
async fn get_data() -> String {
    let data = fetch_data().await;
    
    // Return as TOON
    ToonEncoder::encode_default(&data)
        .unwrap_or_else(|_| serde_json::to_string(&data).unwrap())
}
```

## Learn More

- [TOON Specification](https://github.com/toon-format/toon)
- [toon-format Crate](https://crates.io/crates/toon-format)
- [Example: toon-format-demo](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/toon-format-demo)

