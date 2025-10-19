# Rust Deep Agents SDK

[![Crates.io](https://img.shields.io/crates/v/agents-runtime.svg)](https://crates.io/crates/agents-runtime)
[![Documentation](https://docs.rs/agents-runtime/badge.svg)](https://docs.rs/agents-runtime)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

A high-performance Rust framework for building AI agents with custom tools, sub-agents, and persistent state management. Built for production use with enterprise-grade features like token tracking, cost monitoring, and human-in-the-loop workflows.

## 🆕 What's New in v0.0.24

- **Streaming Events**: Real-time token-by-token event broadcasting for streaming responses
- **StreamingToken Events**: New `AgentEvent::StreamingToken` variant for live updates
- **Opt-in Streaming**: Broadcasters can enable streaming via `supports_streaming()` method
- **Backward Compatible**: Existing broadcasters work unchanged (streaming disabled by default)
- **Enhanced Streaming**: `handle_message_stream()` now emits events for SSE/WebSocket integrations
- **Example**: New `streaming-events-demo` showing real-time token broadcasting

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
agents-sdk = "0.0.24"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

### Basic Agent

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel, get_default_model};
use agents_macros::tool;
use agents_core::state::AgentStateSnapshot;
use std::sync::Arc;

// Define a tool using the #[tool] macro
#[tool("Adds two numbers together")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create OpenAI configuration
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o-mini"
    );

    // Create the model
    let model = Arc::new(OpenAiChatModel::new(config)?);

    // Build an agent with tools
    let agent = ConfigurableAgentBuilder::new("You are a helpful math assistant.")
        .with_model(model)
        .with_tool(AddTool::as_tool())
        .build()?;

    // Use the agent
    let response = agent.handle_message(
        "What is 5 + 3?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or("No response"));

    Ok(())
}
```

## Core Features

### 🤖 Agent Builder API

The `ConfigurableAgentBuilder` provides a fluent interface for constructing agents:

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel, AnthropicConfig, AnthropicMessagesModel, GeminiConfig, GeminiChatModel};
use std::sync::Arc;

// OpenAI
let config = OpenAiConfig::new(api_key, "gpt-4o-mini")?;
let model = Arc::new(OpenAiChatModel::new(config)?);
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .build()?;

// Anthropic
let config = AnthropicConfig::new(api_key, "claude-3-5-sonnet-20241022")?;
let model = Arc::new(AnthropicMessagesModel::new(config)?);
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .build()?;

// Gemini
let config = GeminiConfig::new(api_key, "gemini-2.0-flash-exp")?;
let model = Arc::new(GeminiChatModel::new(config)?);
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .build()?;
```

### 🛠️ Tool System

Define tools using the `#[tool]` macro:

```rust
use agents_macros::tool;

// Simple synchronous tool
#[tool("Multiplies two numbers")]
fn multiply(a: f64, b: f64) -> f64 {
    a * b
}

// Async tool
#[tool("Fetches user data from API")]
async fn get_user(user_id: String) -> String {
    // Make API call...
    format!("User {}", user_id)
}

// Tool with optional parameters
#[tool("Searches with optional filters")]
fn search(query: String, max_results: Option<u32>) -> Vec<String> {
    let limit = max_results.unwrap_or(10);
    // Perform search...
    vec![]
}

// Use the tools
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_tools(vec![
        MultiplyTool::as_tool(),
        GetUserTool::as_tool(),
        SearchTool::as_tool(),
    ])
    .build()?;
```

### 💾 State Persistence

Choose from multiple persistence backends:

```rust
use agents_sdk::{ConfigurableAgentBuilder, InMemoryCheckpointer, RedisCheckpointer};

// In-memory (development)
let checkpointer = Arc::new(InMemoryCheckpointer::new());

// Redis (production)
let checkpointer = Arc::new(
    RedisCheckpointer::new("redis://127.0.0.1:6379").await?
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;

// Save and load state across sessions
let thread_id = "user-123";
agent.save_state(&thread_id).await?;
agent.load_state(&thread_id).await?;
```

### 📊 Token Tracking & Cost Monitoring

Monitor LLM usage and costs with built-in token tracking:

```rust
use agents_sdk::{ConfigurableAgentBuilder, TokenTrackingConfig, TokenCosts};

// Enable token tracking with default settings
let model = Arc::new(OpenAiChatModel::new(config)?);
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_token_tracking(true)  // Enable with defaults
    .build()?;

// Or configure with custom settings
let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};

let model = Arc::new(OpenAiChatModel::new(config)?);
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_token_tracking_config(token_config)
    .build()?;
```

**Features:**
- Real-time token usage tracking
- Cost estimation with predefined pricing models
- Performance metrics (duration, throughput)
- Event broadcasting integration
- Flexible configuration options

### 🔒 Human-in-the-Loop (HITL)

Require human approval for critical operations:

```rust
use agents_sdk::{ConfigurableAgentBuilder, HitlPolicy};
use std::collections::HashMap;

// Configure HITL policies
let mut policies = HashMap::new();
policies.insert(
    "delete_file".to_string(),
    HitlPolicy {
        allow_auto: false,
        note: Some("File deletion requires security review".to_string()),
    }
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_tool_interrupts(policies)
    .with_checkpointer(checkpointer)  // Required for HITL
    .build()?;

// Handle interrupts
match agent.handle_message("Delete the old_data.txt file", state).await {
    Ok(response) => {
        // Check if execution was paused
        if let Some(text) = response.content.as_text() {
            if text.contains("paused") || text.contains("approval") {
                // HITL was triggered!
                if let Some(interrupt) = agent.current_interrupt().await? {
                    println!("Tool: {}", interrupt.tool_name);
                    println!("Args: {}", interrupt.tool_args);
                }
            }
        }
    }
    Err(e) => println!("Error: {}", e),
}

// Resume with approval
agent.resume_with_approval(HitlAction::Accept).await?;
```

### 📡 Event System

Real-time progress tracking and multi-channel notifications:

```rust
use agents_sdk::{ConfigurableAgentBuilder, EventBroadcaster};
use agents_core::events::AgentEvent;
use async_trait::async_trait;

struct ConsoleLogger;

#[async_trait]
impl EventBroadcaster for ConsoleLogger {
    fn id(&self) -> &str { "console" }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::AgentStarted(e) => println!("🚀 Agent started: {}", e.agent_name),
            AgentEvent::ToolStarted(e) => println!("🔧 Tool started: {}", e.tool_name),
            AgentEvent::ToolCompleted(e) => println!("✅ Tool completed: {}", e.tool_name),
            AgentEvent::TokenUsage(e) => println!("📊 Token usage: ${:.4}", e.usage.estimated_cost),
            _ => {}
        }
        Ok(())
    }
}

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_event_broadcaster(Arc::new(ConsoleLogger))
    .build()?;
```

### 🔐 Security & PII Protection

Built-in security features prevent PII leakage:

```rust
// PII sanitization is enabled by default
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .build()?;

// Events automatically have:
// - Message previews truncated to 100 characters
// - Sensitive fields (passwords, tokens, etc.) redacted
// - PII patterns (emails, phones, credit cards) removed

// Disable only if you need raw data and have other security measures
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_pii_sanitization(false)  // Not recommended for production
    .build()?;
```

## Advanced Features

### Sub-Agents

Delegate tasks to specialized sub-agents:

```rust
use agents_sdk::{ConfigurableAgentBuilder, SubAgentConfig};

let subagent = SubAgentConfig {
    name: "data-processor".to_string(),
    description: "Processes complex data with custom logic".to_string(),
    instructions: "You are a data processing specialist.".to_string(),
    tools: vec![/* specialized tools */],
};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_subagent(subagent)
    .with_auto_general_purpose(true)  // Enable automatic delegation
    .build()?;
```

### Built-in Tools

The SDK includes useful built-in tools:

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_builtin_tools(vec![
        "write_todos".to_string(),
        "ls".to_string(),
        "read_file".to_string(),
        "write_file".to_string(),
    ])
    .build()?;
```

### Prompt Caching

Optimize performance with prompt caching:

```rust
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_prompt_caching(true)
    .build()?;
```

## Examples

The SDK includes comprehensive examples:

- [`simple-agent`](examples/simple-agent) - Basic agent with OpenAI
- [`token-tracking-demo`](examples/token-tracking-demo) - Token usage monitoring
- [`hitl-financial-advisor`](examples/hitl-financial-advisor) - Human-in-the-loop workflows
- [`event-system-demo`](examples/event-system-demo) - Event broadcasting
- [`checkpointer-demo`](examples/checkpointer-demo) - State persistence
- [`subagent-demo`](examples/subagent-demo) - Sub-agent delegation

Run examples:

```bash
# Clone the repository
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

# Run a specific example
cargo run --example simple-agent
cargo run --example token-tracking-demo
```

## Architecture

### Workspace Layout

- `crates/agents-core` - Core traits, message structures, and state models
- `crates/agents-runtime` - Runtime engine, builders, and middleware
- `crates/agents-toolkit` - Built-in tools and utilities
- `crates/agents-aws` - AWS integrations (DynamoDB, Secrets Manager)
- `crates/agents-persistence` - Persistence backends (Redis, PostgreSQL)
- `crates/agents-sdk` - Unified SDK with feature flags
- `examples/` - Working examples and demos
- `docs/` - Documentation and guides

### Middleware Stack

The SDK includes a powerful middleware system:

- **Planning Middleware**: Todo list management
- **Filesystem Middleware**: Mock filesystem operations
- **SubAgent Middleware**: Task delegation
- **HITL Middleware**: Human approval workflows
- **Token Tracking Middleware**: Usage and cost monitoring
- **Summarization Middleware**: Context window management
- **PII Sanitization**: Automatic data protection

### Provider Support

- **OpenAI**: GPT models (gpt-4o, gpt-4o-mini, gpt-4-turbo, gpt-3.5-turbo)
- **Anthropic**: Claude models (claude-3-5-sonnet-20241022, claude-3-haiku-20240307)
- **Gemini**: Google's Gemini models (gemini-2.0-flash-exp, gemini-1.5-pro)

## Development

### Building from Source

```bash
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

# Format, lint, and test
cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all

# Build release
cargo build --release
```

### Feature Flags

The SDK supports feature flags for modular functionality:

```toml
[dependencies]
agents-sdk = { version = "0.0.23", features = ["aws", "redis"] }

# Available features:
# - "aws" - AWS integrations (DynamoDB, Secrets Manager)
# - "redis" - Redis persistence backend
# - "postgres" - PostgreSQL persistence backend
# - "dynamodb" - DynamoDB persistence backend
# - "full" - All features enabled
```

### Environment Variables

Required environment variables:

```bash
# OpenAI
export OPENAI_API_KEY="your-openai-api-key"

# Anthropic
export ANTHROPIC_API_KEY="your-anthropic-api-key"

# Gemini
export GOOGLE_API_KEY="your-google-api-key"

# Optional: Tavily for web search
export TAVILY_API_KEY="your-tavily-api-key"
```

## Performance

The Rust SDK is designed for high performance:

- **Memory Efficient**: Zero-copy message handling where possible
- **Async First**: Built on Tokio for concurrent operations
- **Type Safe**: Compile-time guarantees for agent configurations
- **Fast Compilation**: Optimized build times with feature flags
- **Low Latency**: Minimal overhead for tool calls and state management

## Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### Development Setup

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests
5. Run `cargo fmt` and `cargo clippy`
6. Submit a pull request

## License

This project is licensed under the Apache License 2.0 - see the [LICENSE](LICENSE) file for details.

## Support

- 📖 [Documentation](https://docs.rs/agents-runtime)
- 🐛 [Issue Tracker](https://github.com/yafatek/rust-deep-agents-sdk/issues)
- 💬 [Discussions](https://github.com/yafatek/rust-deep-agents-sdk/discussions)

## Roadmap

- [ ] Custom sub-agent execution graphs
- [ ] Dict-based model configuration
- [ ] Advanced state features (encryption, migrations)
- [ ] Enhanced tool system (composition, validation)
- [ ] Performance optimizations
- [ ] Additional LLM providers

---

**Built with ❤️ in Rust for the AI community**