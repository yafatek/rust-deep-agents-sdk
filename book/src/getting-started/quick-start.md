# Quick Start

Build a working AI agent in under 5 minutes.

## Prerequisites

Make sure you have:
- Rust installed
- An OpenAI API key (or Anthropic/Gemini)
- Completed the [Installation](./installation.md) steps

## Create a New Project

```bash
cargo new my-agent
cd my-agent
```

## Add Dependencies

Edit `Cargo.toml`:

```toml
[package]
name = "my-agent"
version = "0.1.0"
edition = "2021"

[dependencies]
agents-sdk = "0.0.29"
tokio = { version = "1.0", features = ["full"] }
anyhow = "1.0"
```

## Write Your Agent

Replace `src/main.rs` with:

```rust
use agents_sdk::{
    ConfigurableAgentBuilder, 
    OpenAiConfig, 
    OpenAiChatModel,
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

// Define a simple tool
#[tool("Greet someone by name")]
fn greet(name: String) -> String {
    format!("Hello, {}! Welcome to the Deep Agents SDK!", name)
}

// Define a calculator tool
#[tool("Add two numbers together")]
fn add(a: i32, b: i32) -> i32 {
    a + b
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Load API key from environment
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("Please set OPENAI_API_KEY environment variable");

    // Configure the LLM
    let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
    let model = Arc::new(OpenAiChatModel::new(config)?);

    // Build the agent with tools
    let agent = ConfigurableAgentBuilder::new(
        "You are a friendly assistant. Use your tools to help users."
    )
    .with_model(model)
    .with_tool(GreetTool::as_tool())
    .with_tool(AddTool::as_tool())
    .build()?;

    // Create a conversation state
    let state = Arc::new(AgentStateSnapshot::default());

    // Send a message
    println!("You: Please greet Alice and then calculate 5 + 3");
    
    let response = agent.handle_message(
        "Please greet Alice and then calculate 5 + 3",
        state
    ).await?;

    println!("Agent: {}", response.content.as_text().unwrap_or("No response"));

    Ok(())
}
```

## Run It

```bash
export OPENAI_API_KEY="your-api-key-here"
cargo run
```

## Expected Output

```
You: Please greet Alice and then calculate 5 + 3
Agent: Hello, Alice! Welcome to the Deep Agents SDK! And 5 + 3 equals 8.
```

## What Just Happened?

1. **Tool Definition**: The `#[tool]` macro turned your Rust functions into tools the agent can use. It automatically:
   - Generated a JSON Schema for the parameters
   - Created wrapper code for the LLM to call

2. **Agent Creation**: `ConfigurableAgentBuilder` assembled the agent with:
   - Instructions defining its personality
   - A language model (GPT-4o-mini)
   - Tools it can use

3. **Message Handling**: When you sent a message:
   - The agent understood it needed to greet someone and do math
   - It called the `greet` tool with "Alice"
   - It called the `add` tool with 5 and 3
   - It composed a natural response

## Try Different Providers

### Anthropic Claude

```rust
use agents_sdk::{AnthropicConfig, AnthropicMessagesModel};

let config = AnthropicConfig::new(
    std::env::var("ANTHROPIC_API_KEY")?,
    "claude-sonnet-4.5",
    4096  // max tokens
);
let model = Arc::new(AnthropicMessagesModel::new(config)?);
```

### Google Gemini

```rust
use agents_sdk::{GeminiConfig, GeminiChatModel};

let config = GeminiConfig::new(
    std::env::var("GOOGLE_API_KEY")?,
    "gemini-2.5-pro"
);
let model = Arc::new(GeminiChatModel::new(config)?);
```

## Add Persistence

Keep conversation history across restarts:

```rust
use agents_sdk::persistence::InMemoryCheckpointer;

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    .build()?;

// Save state
agent.save_state("user-123").await?;

// Later, restore state
agent.load_state("user-123").await?;
```

## Next Steps

Now that you have a working agent:

- [Your First Agent](./first-agent.md) - Deeper dive into agent construction
- [Tools](../concepts/tools.md) - Learn the full tool system
- [Token Tracking](../features/token-tracking.md) - Monitor costs
- [Human-in-the-Loop](../features/hitl.md) - Add approval workflows

