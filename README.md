<p align="center">
  <img src="https://raw.githubusercontent.com/yafatek/rust-deep-agents-sdk/main/docs/assets/logo.svg" alt="Rust Deep Agents SDK" width="400">
</p>

<h1 align="center">Rust Deep Agents SDK</h1>

<p align="center">
  <strong>Build production-ready AI agents in Rust with type safety, blazing performance, and enterprise features.</strong>
</p>

<p align="center">
  <a href="https://crates.io/crates/agents-sdk"><img src="https://img.shields.io/crates/v/agents-sdk.svg?style=flat-square&logo=rust" alt="Crates.io"></a>
  <a href="https://docs.rs/agents-sdk"><img src="https://img.shields.io/docsrs/agents-sdk?style=flat-square&logo=docs.rs" alt="docs.rs"></a>
  <a href="https://github.com/yafatek/rust-deep-agents-sdk/actions"><img src="https://img.shields.io/github/actions/workflow/status/yafatek/rust-deep-agents-sdk/release.yml?style=flat-square&logo=github" alt="Build Status"></a>
  <a href="https://github.com/yafatek/rust-deep-agents-sdk/blob/main/LICENSE"><img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg?style=flat-square" alt="License"></a>
  <a href="https://github.com/yafatek/rust-deep-agents-sdk/stargazers"><img src="https://img.shields.io/github/stars/yafatek/rust-deep-agents-sdk?style=flat-square&logo=github" alt="GitHub Stars"></a>
</p>

<p align="center">
  <a href="#quick-start">Quick Start</a> •
  <a href="#features">Features</a> •
  <a href="#examples">Examples</a> •
  <a href="#why-rust-deep-agents">Why This SDK?</a> •
  <a href="CONTRIBUTING.md">Contributing</a> •
  <a href="https://docs.rs/agents-sdk">Documentation</a>
</p>

---

## Why Rust Deep Agents?

Building AI agents shouldn't mean sacrificing performance or type safety. While Python frameworks dominate the AI space, **Rust Deep Agents SDK** brings the reliability and speed of Rust to agent development.

### Comparison with Alternatives

| Feature | Rust Deep Agents | LangChain | CrewAI | AutoGen |
|---------|------------------|-----------|--------|---------|
| **Language** | Rust | Python | Python | Python |
| **Type Safety** | Compile-time | Runtime | Runtime | Runtime |
| **Performance** | Native speed | Interpreted | Interpreted | Interpreted |
| **Memory Safety** | Guaranteed | GC-dependent | GC-dependent | GC-dependent |
| **Async/Concurrent** | Tokio-native | asyncio | asyncio | asyncio |
| **Tool Macro** | `#[tool]` | Decorators | Decorators | Manual |
| **Token Tracking** | Built-in | Callbacks | Manual | Manual |
| **HITL Workflows** | Native | Plugin | Limited | Plugin |
| **PII Protection** | Automatic | Manual | Manual | Manual |

### Ideal Use Cases

- **Enterprise applications** requiring reliability and compliance
- **High-throughput systems** processing thousands of agent requests
- **Security-critical environments** where memory safety matters
- **Cloud-native deployments** on AWS Lambda, ECS, or Kubernetes
- **Rust teams** who want AI capabilities without Python dependencies

---

## Features

### Multi-Provider LLM Support

The SDK is **model-agnostic** — pass any model string supported by the provider:

- **OpenAI**: Any model (e.g., `gpt-5.2`, `gpt-4o`, `o1-pro`, `gpt-4o-mini`)
- **Anthropic**: Any model (e.g., `claude-opus-4.5`, `claude-sonnet-4.5`, `claude-haiku-4.5`)
- **Google Gemini**: Any model (e.g., `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash`)

### Ergonomic Tool System

- `#[tool]` macro for zero-boilerplate tool definitions
- Automatic JSON Schema generation
- Support for sync and async tools

### Flexible Persistence

- In-memory storage for development
- Redis for production deployments
- PostgreSQL for enterprise environments
- DynamoDB for AWS-native architectures

### Token Tracking and Cost Control

- Real-time usage monitoring
- Cost estimation per request
- Budget alerts and limits

### Human-in-the-Loop (HITL)

- Configurable approval workflows
- Tool-level interrupt policies
- Audit trail for compliance

### Enterprise Security

- Automatic PII sanitization
- Sensitive field redaction
- Compliance-ready event logging

---

## Quick Start

### Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
agents-sdk = "0.0.28"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

### Your First Agent

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
use agents_sdk::state::AgentStateSnapshot;
use agents_macros::tool;
use std::sync::Arc;

// Define a tool with a simple macro
#[tool("Adds two numbers together")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Configure the LLM
    let config = OpenAiConfig::new(
        std::env::var("OPENAI_API_KEY")?,
        "gpt-4o-mini"
    );
    let model = Arc::new(OpenAiChatModel::new(config)?);

    // Build your agent
    let agent = ConfigurableAgentBuilder::new("You are a helpful math assistant.")
        .with_model(model)
        .with_tool(AddTool::as_tool())
        .build()?;

    // Use it
    let response = agent.handle_message(
        "What is 5 + 3?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    
    println!("{}", response.content.as_text().unwrap_or("No response"));
    Ok(())
}
```

---

## Examples

Explore comprehensive examples demonstrating SDK capabilities:

| Example | Description | Complexity |
|---------|-------------|------------|
| [`simple-agent`](examples/simple-agent) | Basic agent with OpenAI | Beginner |
| [`tool-test`](examples/tool-test) | Custom tools with `#[tool]` macro | Beginner |
| [`anthropic-tools-test`](examples/anthropic-tools-test) | Using Claude models | Beginner |
| [`gemini-tools-test`](examples/gemini-tools-test) | Using Gemini models | Beginner |
| [`token-tracking-demo`](examples/token-tracking-demo) | Monitor usage and costs | Intermediate |
| [`event-system-demo`](examples/event-system-demo) | Real-time event broadcasting | Intermediate |
| [`checkpointer-demo`](examples/checkpointer-demo) | State persistence | Intermediate |
| [`hitl-demo`](examples/hitl-demo) | Human-in-the-loop basics | Intermediate |
| [`hitl-financial-advisor`](examples/hitl-financial-advisor) | Production HITL workflow | Advanced |
| [`subagent-demo`](examples/subagent-demo) | Multi-agent delegation | Advanced |
| [`streaming-events-demo`](examples/streaming-events-demo) | SSE/WebSocket streaming | Advanced |
| [`automotive-web-service`](examples/automotive-web-service) | Full-stack web application | Advanced |

### Running Examples

```bash
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

export OPENAI_API_KEY="your-key-here"

cargo run -p tool-test
cargo run -p token-tracking-demo
cargo run -p hitl-financial-advisor
```

---

## Architecture

```
rust-deep-agents-sdk/
├── crates/
│   ├── agents-core/        # Core traits, messages, state models
│   ├── agents-runtime/     # Execution engine, builders, middleware
│   ├── agents-toolkit/     # Built-in tools and utilities
│   ├── agents-macros/      # #[tool] procedural macro
│   ├── agents-sdk/         # Unified SDK with feature flags
│   ├── agents-aws/         # AWS integrations (DynamoDB, Secrets)
│   └── agents-persistence/ # Redis, PostgreSQL backends
├── examples/               # Working examples and demos
├── docs/                   # Documentation and guides
└── deploy/                 # Terraform modules for AWS
```

### Provider Support

The SDK is model-agnostic — you can use any model string supported by the provider's API.

| Provider | Example Models | Status |
|----------|----------------|--------|
| **OpenAI** | `gpt-5.2`, `gpt-4o`, `o1-pro`, `o1-mini`, `gpt-4o-mini` | Stable |
| **Anthropic** | `claude-opus-4.5`, `claude-sonnet-4.5`, `claude-haiku-4.5` | Stable |
| **Google Gemini** | `gemini-2.5-pro`, `gemini-2.5-flash`, `gemini-2.0-flash` | Stable |

> **Note**: Model availability depends on your API access. The SDK passes your model string directly to the provider — any model they support will work.

### Middleware Stack

The SDK includes a composable middleware system:

- **Token Tracking** — Usage and cost monitoring
- **HITL** — Human approval workflows
- **Planning** — Todo list management
- **Summarization** — Context window management
- **PII Sanitization** — Automatic data protection
- **SubAgent** — Task delegation to specialized agents

---

## Advanced Usage

<details>
<summary><strong>Multi-Provider Configuration</strong></summary>

```rust
use agents_sdk::{
    ConfigurableAgentBuilder, 
    OpenAiConfig, OpenAiChatModel,
    AnthropicConfig, AnthropicMessagesModel,
    GeminiConfig, GeminiChatModel
};
use std::sync::Arc;

// OpenAI
let openai = Arc::new(OpenAiChatModel::new(
    OpenAiConfig::new(api_key, "gpt-4o-mini")?
)?);

// Anthropic Claude
let claude = Arc::new(AnthropicMessagesModel::new(
    AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096)?
)?);

// Google Gemini
let gemini = Arc::new(GeminiChatModel::new(
    GeminiConfig::new(api_key, "gemini-2.5-pro")?
)?);

// Use any provider with the same builder API
let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(claude)
    .build()?;
```

</details>

<details>
<summary><strong>Token Tracking and Cost Control</strong></summary>

```rust
use agents_sdk::{ConfigurableAgentBuilder, TokenTrackingConfig, TokenCosts};

let token_config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_token_tracking_config(token_config)
    .build()?;
```

</details>

<details>
<summary><strong>Human-in-the-Loop (HITL)</strong></summary>

```rust
use agents_sdk::{ConfigurableAgentBuilder, HitlPolicy};
use std::collections::HashMap;

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
    .with_checkpointer(checkpointer)
    .build()?;

// Handle interrupts
                if let Some(interrupt) = agent.current_interrupt().await? {
    println!("Approval needed for: {}", interrupt.tool_name);
    agent.resume_with_approval(HitlAction::Accept).await?;
}
```

</details>

<details>
<summary><strong>State Persistence</strong></summary>

```rust
use agents_sdk::{ConfigurableAgentBuilder, InMemoryCheckpointer};
use agents_persistence::RedisCheckpointer;
use std::sync::Arc;

// Development: In-memory
let checkpointer = Arc::new(InMemoryCheckpointer::new());

// Production: Redis
let checkpointer = Arc::new(
    RedisCheckpointer::new("redis://127.0.0.1:6379").await?
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;

// Save and restore conversation state
let thread_id = "user-123";
agent.save_state(&thread_id).await?;
agent.load_state(&thread_id).await?;
```

</details>

<details>
<summary><strong>Event System and Streaming</strong></summary>

```rust
use agents_sdk::{ConfigurableAgentBuilder, EventBroadcaster};
use agents_core::events::AgentEvent;
use async_trait::async_trait;

struct WebhookBroadcaster {
    endpoint: String,
}

#[async_trait]
impl EventBroadcaster for WebhookBroadcaster {
    fn id(&self) -> &str { "webhook" }
    
    fn supports_streaming(&self) -> bool { true }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::AgentStarted(e) => { /* POST to webhook */ }
            AgentEvent::StreamingToken(e) => { /* SSE push */ }
            AgentEvent::ToolCompleted(e) => { /* Log to analytics */ }
            AgentEvent::TokenUsage(e) => { /* Track costs */ }
            _ => {}
        }
        Ok(())
    }
}

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
    .with_model(model)
    .with_event_broadcaster(Arc::new(WebhookBroadcaster { 
        endpoint: "https://api.example.com/events".into() 
    }))
    .build()?;
```

</details>

<details>
<summary><strong>Sub-Agents</strong></summary>

```rust
use agents_sdk::{ConfigurableAgentBuilder, SubAgentConfig};

let researcher = SubAgentConfig {
    name: "researcher".to_string(),
    description: "Searches and analyzes information".to_string(),
    instructions: "You are a research specialist.".to_string(),
    tools: vec![],
};

let writer = SubAgentConfig {
    name: "writer".to_string(),
    description: "Creates well-written content".to_string(),
    instructions: "You are a content writer.".to_string(),
    tools: vec![],
};

let agent = ConfigurableAgentBuilder::new("You are a project coordinator.")
    .with_model(model)
    .with_subagent(researcher)
    .with_subagent(writer)
    .with_auto_general_purpose(true)
    .build()?;
```

</details>

---

## Feature Flags

```toml
[dependencies]
# Default: includes toolkit
agents-sdk = "0.0.28"

# Minimal: core only
agents-sdk = { version = "0.0.28", default-features = false }

# With persistence
agents-sdk = { version = "0.0.28", features = ["redis"] }
agents-sdk = { version = "0.0.28", features = ["postgres"] }

# With AWS
agents-sdk = { version = "0.0.28", features = ["aws", "dynamodb"] }

# Everything
agents-sdk = { version = "0.0.28", features = ["full"] }
```

| Feature | Description |
|---------|-------------|
| `toolkit` | Built-in tools (default) |
| `redis` | Redis persistence backend |
| `postgres` | PostgreSQL persistence backend |
| `dynamodb` | DynamoDB persistence backend |
| `aws` | AWS integrations (Secrets Manager, etc.) |
| `full` | All features enabled |

---

## Development

```bash
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

cargo fmt
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo build --release
```

### Environment Variables

```bash
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GOOGLE_API_KEY="..."
export TAVILY_API_KEY="..."  # Optional: for web search
```

---

## Contributing

We welcome contributions of all kinds:

- Bug reports and fixes
- Feature requests and implementations
- Documentation improvements
- Test coverage

Please read our [Contributing Guide](CONTRIBUTING.md) to get started.

New contributors can look for issues labeled [`good first issue`](https://github.com/yafatek/rust-deep-agents-sdk/labels/good%20first%20issue).

---

## Roadmap

**Providers**
- AWS Bedrock provider (Claude, Titan, Llama)
- Ollama for local/self-hosted models
- Azure OpenAI Service

**Features**
- Custom sub-agent execution graphs
- Advanced state features (encryption, migrations)
- Enhanced tool composition and validation
- WebAssembly support
- OpenTelemetry integration

See the [full roadmap](docs/ROADMAP.md) for details.

---

## License

This project is licensed under the Apache License 2.0. See the [LICENSE](LICENSE) file for details.

---

## Support

- [API Documentation](https://docs.rs/agents-sdk)
- [GitHub Discussions](https://github.com/yafatek/rust-deep-agents-sdk/discussions)
- [Issue Tracker](https://github.com/yafatek/rust-deep-agents-sdk/issues)

---

<p align="center">
  <sub>Built with Rust for production AI systems</sub>
</p>
