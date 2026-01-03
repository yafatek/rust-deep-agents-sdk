# TOON Format Support

The Rust Deep Agents SDK supports [TOON (Token-Oriented Object Notation)](https://github.com/toon-format/toon), a compact, human-readable serialization format designed specifically for LLM prompts. TOON provides **30-60% token reduction** compared to JSON.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [ToonEncoder API](#toonencoder-api)
- [System Prompt Format](#system-prompt-format)
- [Token Savings](#token-savings)
- [When to Use TOON](#when-to-use-toon)
- [Feature Flags](#feature-flags)
- [Examples](#examples)

## Overview

TOON is particularly effective for:
- Arrays of uniform objects (tabular data)
- Tool call examples in system prompts
- Structured tool results

### JSON vs TOON Comparison

**JSON:**
```json
{
  "users": [
    {"id": 1, "name": "Alice", "active": true},
    {"id": 2, "name": "Bob", "active": true},
    {"id": 3, "name": "Charlie", "active": false}
  ]
}
```

**TOON:**
```toon
users[3]{id,name,active}:
  1,Alice,true
  2,Bob,true
  3,Charlie,false
```

## Quick Start

### 1. Enable the Feature

Add to your `Cargo.toml`:

```toml
[dependencies]
agents-runtime = { version = "0.0.29", features = ["toon"] }
agents-core = { version = "0.0.29", features = ["toon"] }
```

### 2. Use TOON Format in Your Agent

```rust
use agents_runtime::{ConfigurableAgentBuilder, PromptFormat};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_prompt_format(PromptFormat::Toon)  // Use TOON-formatted prompts
    .build()?;
```

### 3. Encode Tool Results with ToonEncoder

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

let toon_output = encoder.encode(&data)?;
// Output:
// products[2]{id,name,price}:
//   1,Widget,9.99
//   2,Gadget,19.99
```

## ToonEncoder API

### Creating an Encoder

```rust
use agents_core::toon::ToonEncoder;

// Default encoder
let encoder = ToonEncoder::new();

// Compact encoder (uses tabs, enables key folding)
let compact_encoder = ToonEncoder::compact();

// Custom configuration
let custom_encoder = ToonEncoder::new()
    .with_tabs(true)      // Use tab delimiters
    .with_key_folding(true); // Fold nested keys
```

### Encoding Methods

```rust
// Encode any Serialize type
let result = encoder.encode(&my_struct)?;

// Encode with default options (static method)
let result = ToonEncoder::encode_default(&my_value)?;

// Encode serde_json::Value
let result = encoder.encode_json(&json_value)?;
```

### Utility Functions

```rust
use agents_core::toon::{tool_schema_to_toon, format_tool_call_toon};

// Convert a tool schema to TOON format
let tool = MyTool::as_tool();
let toon_schema = tool_schema_to_toon(&tool.schema())?;

// Format a tool call example
let toon_call = format_tool_call_toon("search", &json!({"query": "rust"}))?;
// Output:
// tool: search
// args:
//   query: rust
```

## System Prompt Format

### PromptFormat Enum

```rust
use agents_runtime::PromptFormat;

// JSON format (default) - Most compatible
PromptFormat::Json

// TOON format - 30-60% token reduction
PromptFormat::Toon
```

### Using with ConfigurableAgentBuilder

```rust
// Default JSON format
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .build()?;

// Explicit TOON format
let agent = ConfigurableAgentBuilder::new("instructions")
    .with_model(model)
    .with_prompt_format(PromptFormat::Toon)
    .build()?;
```

### Using with DeepAgentConfig

```rust
use agents_runtime::agent::config::DeepAgentConfig;
use agents_runtime::PromptFormat;

let config = DeepAgentConfig::new("instructions", planner)
    .with_prompt_format(PromptFormat::Toon);
```

### Direct Prompt Generation

```rust
use agents_runtime::prompts::{
    get_deep_agent_system_prompt,
    get_deep_agent_system_prompt_toon,
    get_deep_agent_system_prompt_formatted,
    PromptFormat,
};

// JSON format
let json_prompt = get_deep_agent_system_prompt("Custom instructions");

// TOON format
let toon_prompt = get_deep_agent_system_prompt_toon("Custom instructions");

// Dynamic format selection
let prompt = get_deep_agent_system_prompt_formatted(
    "Custom instructions",
    PromptFormat::Toon,
);
```

## Token Savings

### Estimated Savings by Data Type

| Data Type | JSON Tokens | TOON Tokens | Savings |
|-----------|-------------|-------------|---------|
| Simple object | 20 | 15 | 25% |
| Array of objects | 100 | 45 | 55% |
| Nested object | 50 | 35 | 30% |
| Tool call example | 30 | 15 | 50% |

### System Prompt Comparison

| Component | JSON | TOON | Savings |
|-----------|------|------|---------|
| Tool examples | ~500 tokens | ~250 tokens | 50% |
| Full system prompt | ~1500 tokens | ~1100 tokens | 27% |

### Cost Savings (GPT-4o-mini)

| Requests | JSON Cost | TOON Cost | Savings |
|----------|-----------|-----------|---------|
| 1,000 | $0.23 | $0.17 | $0.06 |
| 100,000 | $22.50 | $16.50 | $6.00 |
| 1,000,000 | $225.00 | $165.00 | $60.00 |

## When to Use TOON

### Recommended

- **High-volume applications** - Cost savings add up quickly
- **Structured tool results** - Arrays of objects compress well
- **Token-constrained models** - More room for context
- **Production deployments** - Lower costs at scale

### Consider JSON Instead

- **Debugging** - JSON is more widely supported for inspection
- **Compatibility-critical** - Some models may prefer JSON
- **Simple data** - Single values don't benefit much
- **External integrations** - When other systems expect JSON

## Feature Flags

The TOON functionality is feature-gated to avoid adding dependencies when not needed.

### Enabling TOON

```toml
# agents-core with TOON
agents-core = { version = "0.0.29", features = ["toon"] }

# agents-runtime with TOON (automatically enables agents-core/toon)
agents-runtime = { version = "0.0.29", features = ["toon"] }
```

### Feature Behavior

| Feature Enabled | Behavior |
|-----------------|----------|
| Yes | Full TOON encoding with `toon-format` crate |
| No | Falls back to JSON encoding |

## Examples

### Running the Demo

```bash
cd examples/toon-format-demo
export OPENAI_API_KEY=your-api-key
cargo run
```

### Tool with TOON-Encoded Results

```rust
use agents_core::toon::ToonEncoder;
use agents_sdk::tool;
use serde_json::json;

#[tool("Search for products")]
fn search_products(query: String, limit: i32) -> String {
    let results = vec![
        json!({"id": 1, "name": "Product A", "price": 29.99}),
        json!({"id": 2, "name": "Product B", "price": 39.99}),
    ];
    
    let encoder = ToonEncoder::new();
    encoder.encode(&json!({"results": results}))
        .unwrap_or_else(|_| serde_json::to_string(&results).unwrap())
}
```

### Complete Agent Setup

```rust
use agents_runtime::{ConfigurableAgentBuilder, PromptFormat};
use agents_sdk::{OpenAiChatModel, OpenAiConfig, InMemoryCheckpointer};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    
    let agent = ConfigurableAgentBuilder::new(
        "You are a helpful shopping assistant."
    )
    .with_model(Arc::new(OpenAiChatModel::new(config)?))
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .with_prompt_format(PromptFormat::Toon)  // Enable TOON format
    .with_tool(SearchProductsTool::as_tool())
    .build()?;
    
    // Use the agent...
    Ok(())
}
```

## Learn More

- [TOON Format Specification](https://github.com/toon-format/toon)
- [TOON Rust Crate](https://crates.io/crates/toon-format)
- [toon-format-demo Example](../examples/toon-format-demo/)

