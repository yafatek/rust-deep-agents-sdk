# Rust Deep Agents SDK

<div class="hero">
  <img src="https://raw.githubusercontent.com/yafatek/rust-deep-agents-sdk/main/docs/assets/logo.svg" alt="Deep Agents SDK" width="400">
</div>

**Build production-ready AI agents in Rust with type safety, blazing performance, and enterprise features.**

[![Crates.io](https://img.shields.io/crates/v/agents-sdk.svg?style=flat-square&logo=rust)](https://crates.io/crates/agents-sdk)
[![docs.rs](https://img.shields.io/docsrs/agents-sdk?style=flat-square&logo=docs.rs)](https://docs.rs/agents-sdk)
[![Build Status](https://img.shields.io/github/actions/workflow/status/yafatek/rust-deep-agents-sdk/release.yml?style=flat-square&logo=github)](https://github.com/yafatek/rust-deep-agents-sdk/actions)
[![License](https://img.shields.io/badge/license-Apache%202.0-blue.svg?style=flat-square)](https://github.com/yafatek/rust-deep-agents-sdk/blob/main/LICENSE)

---

## What is Deep Agents SDK?

The **Rust Deep Agents SDK** is a high-performance framework for building AI agents that can:

- **Use tools** to interact with external systems (APIs, databases, file systems)
- **Maintain state** across conversations with persistent memory
- **Delegate tasks** to specialized sub-agents
- **Track costs** with built-in token usage monitoring
- **Require approval** for critical operations via Human-in-the-Loop workflows
- **Stream responses** in real-time for interactive applications

All with the safety guarantees, performance, and reliability that Rust provides.

## Why Rust for AI Agents?

| Aspect | Rust Deep Agents | Python Frameworks |
|--------|------------------|-------------------|
| **Type Safety** | Compile-time guarantees | Runtime errors |
| **Performance** | Native speed, zero-cost abstractions | Interpreted, GIL limitations |
| **Memory Safety** | Guaranteed by compiler | Manual management |
| **Concurrency** | Fearless with Tokio | asyncio complexity |
| **Deployment** | Single binary | Virtual environments |
| **Dependencies** | Minimal, audited | Large dependency trees |

## Quick Example

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel, tool};
use std::sync::Arc;

// Define a tool with a simple macro
#[tool("Search for information on the web")]
async fn search(query: String) -> String {
    format!("Results for: {}", query)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(std::env::var("OPENAI_API_KEY")?, "gpt-4o-mini")
    )?);

    let agent = ConfigurableAgentBuilder::new("You are a helpful research assistant.")
        .with_model(model)
        .with_tool(SearchTool::as_tool())
        .build()?;

    let response = agent.handle_message(
        "Find information about Rust programming",
        Arc::new(agents_sdk::state::AgentStateSnapshot::default())
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());
    Ok(())
}
```

## Features at a Glance

### Multi-Provider LLM Support

```rust
// OpenAI
OpenAiConfig::new(api_key, "gpt-4o-mini")

// Anthropic Claude
AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)

// Google Gemini
GeminiConfig::new(api_key, "gemini-2.5-pro")
```

### Ergonomic Tool System

```rust
#[tool("Calculate the sum of two numbers")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}
```

### Flexible Persistence

```rust
// In-memory (development)
Arc::new(InMemoryCheckpointer::new())

// Redis (production)
Arc::new(RedisCheckpointer::new("redis://localhost").await?)

// PostgreSQL (enterprise)
Arc::new(PostgresCheckpointer::new("postgresql://...").await?)

// DynamoDB (AWS-native)
Arc::new(DynamoDbCheckpointer::new("table-name").await?)
```

### Human-in-the-Loop Workflows

```rust
let mut policies = HashMap::new();
policies.insert("delete_file".to_string(), HitlPolicy {
    allow_auto: false,
    note: Some("Requires approval".to_string()),
});

let agent = ConfigurableAgentBuilder::new("You are an assistant")
    .with_tool_interrupts(policies)
    .build()?;
```

### Token Tracking & Cost Control

```rust
let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
    ..Default::default()
};

let agent = ConfigurableAgentBuilder::new("You are an assistant")
    .with_token_tracking_config(token_config)
    .build()?;
```

## Getting Started

Ready to build your first agent? Head to the [Installation](./getting-started/installation.md) guide.

## Community & Support

- [GitHub Repository](https://github.com/yafatek/rust-deep-agents-sdk)
- [GitHub Discussions](https://github.com/yafatek/rust-deep-agents-sdk/discussions)
- [Issue Tracker](https://github.com/yafatek/rust-deep-agents-sdk/issues)
- [API Documentation (docs.rs)](https://docs.rs/agents-sdk)

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](https://github.com/yafatek/rust-deep-agents-sdk/blob/main/LICENSE) file for details.

