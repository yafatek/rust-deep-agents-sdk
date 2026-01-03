# Installation

This guide walks you through adding the Rust Deep Agents SDK to your project.

## Prerequisites

- **Rust 1.70+** (2021 edition)
- **Tokio runtime** (async support)
- **API key** for your LLM provider (OpenAI, Anthropic, or Google)

## Add to Your Project

Add the SDK to your `Cargo.toml`:

```toml
[dependencies]
agents-sdk = "0.0.29"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

## Installation Options

The SDK uses feature flags to minimize dependencies. Choose what you need:

### Default Installation (Recommended)

Includes the toolkit with built-in tools and the `#[tool]` macro:

```toml
agents-sdk = "0.0.29"
```

### Minimal Installation

Core functionality only, smallest binary size:

```toml
agents-sdk = { version = "0.0.29", default-features = false }
```

### With Persistence Backends

```toml
# Redis
agents-sdk = { version = "0.0.29", features = ["redis"] }

# PostgreSQL
agents-sdk = { version = "0.0.29", features = ["postgres"] }

# DynamoDB (AWS)
agents-sdk = { version = "0.0.29", features = ["dynamodb"] }

# All persistence backends
agents-sdk = { version = "0.0.29", features = ["persistence"] }
```

### With AWS Integrations

```toml
# AWS (Secrets Manager, etc.)
agents-sdk = { version = "0.0.29", features = ["aws"] }

# AWS with DynamoDB
agents-sdk = { version = "0.0.29", features = ["aws-full"] }
```

### With TOON Format (Token Optimization)

```toml
agents-sdk = { version = "0.0.29", features = ["toon"] }
```

### Everything Included

```toml
agents-sdk = { version = "0.0.29", features = ["full"] }
```

## Feature Reference

| Feature | Description | Dependencies |
|---------|-------------|--------------|
| `toolkit` | Built-in tools and `#[tool]` macro (default) | `agents-toolkit`, `agents-macros` |
| `toon` | TOON format for token-efficient prompts | `toon-format` |
| `redis` | Redis persistence backend | `redis` |
| `postgres` | PostgreSQL persistence backend | `sqlx` |
| `dynamodb` | DynamoDB persistence backend | `aws-sdk-dynamodb` |
| `aws` | AWS integrations (Secrets Manager) | `aws-config`, `aws-sdk-*` |
| `persistence` | All persistence backends | `redis`, `postgres` |
| `aws-full` | AWS with DynamoDB | `aws`, `dynamodb` |
| `full` | All features enabled | Everything |

## Environment Setup

Set up your API keys as environment variables:

```bash
# OpenAI
export OPENAI_API_KEY="sk-..."

# Anthropic
export ANTHROPIC_API_KEY="sk-ant-..."

# Google Gemini
export GOOGLE_API_KEY="..."

# Optional: Web search
export TAVILY_API_KEY="..."
```

Or use a `.env` file with the `dotenvy` crate:

```toml
[dependencies]
dotenvy = "0.15"
```

```rust
fn main() {
    dotenvy::dotenv().ok();
    let api_key = std::env::var("OPENAI_API_KEY")?;
}
```

## Verify Installation

Create a simple test to verify everything works:

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o-mini"
    );
    
    let model = Arc::new(OpenAiChatModel::new(config)?);
    
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .build()?;
    
    println!("✅ Agent created successfully!");
    Ok(())
}
```

Run it:

```bash
cargo run
```

If you see "✅ Agent created successfully!", you're ready to go!

## Next Steps

- [Quick Start](./quick-start.md) - Build your first working agent
- [Your First Agent](./first-agent.md) - Detailed walkthrough with tools
- [Configuration](./configuration.md) - All builder options explained

