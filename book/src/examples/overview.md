# Examples Overview

The SDK includes comprehensive examples demonstrating various features.

## Running Examples

```bash
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

export OPENAI_API_KEY="your-key-here"

cargo run -p <example-name>
```

## Examples by Complexity

### Beginner

| Example | Description | Key Concepts |
|---------|-------------|--------------|
| [`simple-agent`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/simple-agent) | Basic agent setup | ConfigurableAgentBuilder |
| [`tool-test`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/tool-test) | Custom tools | `#[tool]` macro |
| [`anthropic-tools-test`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/anthropic-tools-test) | Claude models | AnthropicConfig |
| [`gemini-tools-test`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/gemini-tools-test) | Gemini models | GeminiConfig |

### Intermediate

| Example | Description | Key Concepts |
|---------|-------------|--------------|
| [`token-tracking-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/token-tracking-demo) | Usage monitoring | TokenTrackingConfig |
| [`event-system-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/event-system-demo) | Event broadcasting | EventDispatcher |
| [`checkpointer-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/checkpointer-demo) | State persistence | Checkpointer |
| [`hitl-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/hitl-demo) | Approval workflows | HitlPolicy |
| [`toon-format-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/toon-format-demo) | Token optimization | PromptFormat::Toon |

### Advanced

| Example | Description | Key Concepts |
|---------|-------------|--------------|
| [`hitl-financial-advisor`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/hitl-financial-advisor) | Production HITL | Full workflow |
| [`subagent-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/subagent-demo) | Task delegation | SubAgentConfig |
| [`streaming-events-demo`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/streaming-events-demo) | Real-time streaming | SSE/WebSocket |
| [`automotive-web-service`](https://github.com/yafatek/rust-deep-agents-sdk/tree/main/examples/automotive-web-service) | Full web app | Axum + React |

## Examples by Feature

### Multi-Provider

- `anthropic-tools-test` - Anthropic Claude
- `gemini-tools-test` - Google Gemini
- `simple-agent` - OpenAI

### Persistence

- `checkpointer-demo` - In-memory
- `state-persistence-test` - Redis/PostgreSQL

### Real-Time

- `streaming-events-demo` - Token streaming
- `event-system-demo` - Event broadcasting

### Production Patterns

- `hitl-financial-advisor` - Approval workflows
- `automotive-web-service` - Full-stack application
- `deep-agent-server` - API server

## Quick Example Reference

### Minimal Agent

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
use std::sync::Arc;

let model = Arc::new(OpenAiChatModel::new(
    OpenAiConfig::new(api_key, "gpt-4o-mini")
)?);

let agent = ConfigurableAgentBuilder::new("You are helpful.")
    .with_model(model)
    .build()?;
```

### With Tools

```rust
#[tool("Add numbers")]
fn add(a: i32, b: i32) -> i32 { a + b }

let agent = ConfigurableAgentBuilder::new("Math assistant")
    .with_model(model)
    .with_tool(AddTool::as_tool())
    .build()?;
```

### With Persistence

```rust
let agent = ConfigurableAgentBuilder::new("Persistent assistant")
    .with_model(model)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .build()?;
```

### With HITL

```rust
let mut policies = HashMap::new();
policies.insert("delete".to_string(), HitlPolicy { allow_auto: false, note: None });

let agent = ConfigurableAgentBuilder::new("Safe assistant")
    .with_model(model)
    .with_tool_interrupts(policies)
    .with_checkpointer(checkpointer)
    .build()?;
```

