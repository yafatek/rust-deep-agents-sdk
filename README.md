# Rust Deep Agents SDK

[![Crates.io](https://img.shields.io/crates/v/agents-runtime.svg)](https://crates.io/crates/agents-runtime)
[![Documentation](https://docs.rs/agents-runtime/badge.svg)](https://docs.rs/agents-runtime)
[![License](https://img.shields.io/crates/l/agents-runtime.svg)](LICENSE)

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

Add the following to your `Cargo.toml`:

```toml
[dependencies]
agents-runtime = "0.0.1"
agents-core = "0.0.1"
agents-toolkit = "0.0.1"
# Optional: AWS integrations
agents-aws = "0.0.1"
```

## Quick Start

### Using the Published Crates

```rust
use agents_runtime::{ConfigurableAgentBuilder, get_default_model};
use agents_toolkit::create_tool;
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
    let response = agent.run("Please greet Alice using the greet tool").await?;
    println!("{}", response);

    Ok(())
}
```

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
- **Persistence**: `Checkpointer` trait with `InMemoryCheckpointer` implementation
- **Thread Management**: Save/load/delete agent conversation threads

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

## Next Steps
Follow the [roadmap](docs/ROADMAP.md) to implement planners, runtime orchestration, AWS integrations, and customer-ready templates.
