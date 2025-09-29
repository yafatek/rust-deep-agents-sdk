# Rust Deep Agents SDK

[![Crates.io](https://img.shields.io/crates/v/agents-runtime.svg)](https://crates.io/crates/agents-runtime)
[![Documentation](https://docs.rs/agents-runtime/badge.svg)](https://docs.rs/agents-runtime)
[![License](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)

High-performance Rust framework for composing reusable "deep" AI agents with custom tools, sub-agents, and prompts. This repository contains the SDK workspace, AWS integration helpers, documentation, and deployment scaffolding.

## Workspace Layout
- `crates/agents-core` – Domain traits, message structures, prompt packs, and state models.
- `crates/agents-runtime` – Tokio-powered runtime glue between planners, tools, and state stores.
- `crates/agents-toolkit` – Built-in tools (mock filesystem, todo management) and utilities.
- `crates/agents-aws` – AWS adapters (Secrets Manager, DynamoDB, CloudWatch) behind feature flags.
- `examples/` – Reference agents; `getting-started` provides the echo smoke test.
  - `agents-example-cli` provides a local CLI harness using OpenAI.
- `deploy/` – Terraform modules and IaC assets for AWS environments.
- `docs/` – Roadmap, ADRs, playbooks, and reference material.

## Installation

Add the unified SDK to your `Cargo.toml`:

```toml
# Simple installation (includes toolkit by default)
[dependencies]
agents-sdk = "0.0.1"

# Or choose specific features:
# agents-sdk = { version = "0.0.1", default-features = false }  # Core only
# agents-sdk = { version = "0.0.1", features = ["aws"] }       # With AWS
# agents-sdk = { version = "0.0.1", features = ["redis"] }     # With Redis persistence
# agents-sdk = { version = "0.0.1", features = ["postgres"] }  # With PostgreSQL persistence
# agents-sdk = { version = "0.0.1", features = ["dynamodb"] }  # With DynamoDB persistence
# agents-sdk = { version = "0.0.1", features = ["full"] }      # Everything
```

### Individual Crates (Advanced)

If you prefer granular control, you can also use individual crates:

```toml
[dependencies]
agents-core = "0.0.1"      # Core traits and types
agents-runtime = "0.0.1"   # Agent runtime and builders
agents-toolkit = "0.0.1"   # Built-in tools (optional)
agents-aws = "0.0.1"       # AWS integrations (optional)
```

## Quick Start

### Using the Published Crates

```rust
use agents_sdk::{ConfigurableAgentBuilder, get_default_model, create_tool};
use serde_json::Value;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Create a simple tool
    let my_tool = create_tool(
        "greet",
        "Greets a person by name",
        |args: Value| async move {
            let name = args.get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("World");
            Ok(format!("Hello, {}!", name))
        }
    );

    // Build an agent with the default Claude model
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(get_default_model())
        .with_tool(my_tool)
        .build()
        .await?;

    // Use the agent
    use agents_sdk::state::AgentStateSnapshot;
    use std::sync::Arc;

    let response = agent.handle_message(
        "Please greet Alice using the greet tool",
        Arc::new(AgentStateSnapshot::default())
    ).await?;
    println!("{:?}", response);

    Ok(())
}
```

### Using Persistence Backends

Choose the persistence layer that fits your infrastructure:

```rust
use agents_sdk::{ConfigurableAgentBuilder, InMemoryCheckpointer};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // InMemory (default, no external dependencies)
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    
    // Redis (requires redis feature)
    #[cfg(feature = "redis")]
    let checkpointer = Arc::new(
        agents_sdk::RedisCheckpointer::new("redis://127.0.0.1:6379").await?
    );
    
    // PostgreSQL (requires postgres feature)
    #[cfg(feature = "postgres")]
    let checkpointer = Arc::new(
        agents_sdk::PostgresCheckpointer::new("postgresql://user:pass@localhost/agents").await?
    );
    
    // DynamoDB (requires dynamodb feature)
    #[cfg(feature = "dynamodb")]
    let checkpointer = Arc::new(
        agents_sdk::DynamoDbCheckpointer::new("agent-checkpoints").await?
    );

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
        .with_checkpointer(checkpointer)
        .build()?;

    // Save and load state across sessions
    let thread_id = "user-123";
    agent.save_state(thread_id).await?;
    agent.load_state(thread_id).await?;
    
    Ok(())
}
```

See [`examples/checkpointer-demo`](examples/checkpointer-demo) for a complete working example.

### Development Setup (From Source)

```bash
git clone https://github.com/yafatek/rust-deep-agents-sdk.git
cd rust-deep-agents-sdk

# Format, lint, and test
cargo fmt
cargo clippy --all-targets --all-features
cargo test --all

# Run examples
cargo run --example simple-agent
cargo run --example deep-research-agent
cargo run --example checkpointer-demo
```

## Features

### ✅ Core Features (Python Parity Achieved)

**Agent Builder API**
- `ConfigurableAgentBuilder` with fluent interface matching Python's API
- `.with_model()` method supporting OpenAI, Anthropic, and Gemini models
- `.get_default_model()` function returning pre-configured Claude Sonnet 4
- Async and sync agent creation: `create_deep_agent()` and `create_async_deep_agent()`

**Middleware Stack**
- **Planning Middleware**: Todo list management with comprehensive tool descriptions
- **Filesystem Middleware**: Mock filesystem with `ls`, `read_file`, `write_file`, `edit_file` tools
- **SubAgent Middleware**: Task delegation to specialized sub-agents
- **HITL (Human-in-the-Loop)**: Tool interrupts with approval policies
- **Summarization Middleware**: Context window management
- **AnthropicPromptCaching**: Automatic prompt caching for efficiency

**State Management**
- **State Reducers**: Smart merging functions matching Python's `file_reducer` behavior
- **Persistence**: `Checkpointer` trait with multiple backend implementations
- **Thread Management**: Save/load/delete agent conversation threads

**Persistence Backends**
- **InMemory**: Built-in, zero-config persistence (development)
- **Redis**: High-performance in-memory data store with optional durability
- **PostgreSQL**: ACID-compliant relational database with full SQL support
- **DynamoDB**: AWS-managed NoSQL database with auto-scaling

**Provider Support**
- **Anthropic**: Claude models with prompt caching support
- **OpenAI**: GPT models integration  
- **Gemini**: Google's Gemini Chat models

**Built-in Tools**
- **Todo Management**: `write_todos` with detailed usage examples
- **File Operations**: Full CRUD operations on mock filesystem
- **Task Delegation**: `task` tool for spawning ephemeral sub-agents

### 🚧 Future Features (Planned)

#### Custom SubAgent Support
Enable users to define completely custom execution graphs beyond simple prompt/tool configurations:

```rust
// Future API design
let custom_subagent = CustomSubAgent {
    name: "data-processor".to_string(),
    description: "Processes complex data with custom logic".to_string(),
    graph: Box::new(MyCustomGraph::new()), // Custom execution graph
};

let agent = ConfigurableAgentBuilder::new("main instructions")
    .with_custom_subagent(custom_subagent)
    .build()?;
```

**Benefits:**
- Full control over sub-agent execution flow
- Custom state management within sub-agents  
- Complex branching and conditional logic
- Integration with external systems and APIs

#### Dict-based Model Configuration
Allow models to be configured via dictionary/struct configs in addition to instances:

```rust
// Future API design
let agent = ConfigurableAgentBuilder::new("main instructions")
    .with_model_config(ModelConfig {
        provider: "anthropic".to_string(),
        model: "claude-sonnet-4".to_string(),
        max_tokens: 64000,
        temperature: 0.1,
        // ... other provider-specific options
    })
    .build()?;
```

**Benefits:**
- Simplified configuration management
- Easy serialization/deserialization of agent configs
- Runtime model switching without code changes
- Better integration with configuration management systems

#### Advanced State Features
- **Distributed State Stores**: Redis, DynamoDB backends for multi-agent systems
- **State Migrations**: Schema evolution support for long-running agents
- **State Encryption**: Automatic encryption for sensitive data
- **Custom Reducers**: User-defined state merging logic beyond built-in reducers

#### Enhanced Tool System  
- **Dynamic Tool Registration**: Runtime tool addition/removal
- **Tool Composition**: Combining multiple tools into workflows
- **Tool Validation**: Schema-based input/output validation
- **Tool Metrics**: Performance and usage analytics

## Support the Project

If you find this project helpful, consider supporting its development:

[![PayPal](https://img.shields.io/badge/PayPal-00457C?style=for-the-badge&logo=paypal&logoColor=white)](https://paypal.me/yafacs)

Your support helps maintain and improve this open-source project. Thank you! ❤️

## Next Steps
Follow the [roadmap](docs/ROADMAP.md) to implement planners, runtime orchestration, AWS integrations, and customer-ready templates.
