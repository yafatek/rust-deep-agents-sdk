# Configuration

Complete reference for `ConfigurableAgentBuilder` options.

## Builder Overview

```rust
use agents_sdk::ConfigurableAgentBuilder;

let agent = ConfigurableAgentBuilder::new("Your agent instructions")
    .with_model(model)                    // Required: LLM provider
    .with_tool(tool)                      // Optional: Add tools
    .with_checkpointer(checkpointer)      // Optional: State persistence
    .with_token_tracking_config(config)   // Optional: Cost monitoring
    .with_tool_interrupt("tool", policy)  // Optional: HITL workflows
    .with_subagent_config([subagent])     // Optional: Task delegation
    .with_prompt_format(format)           // Optional: TOON/JSON
    .with_max_iterations(10)              // Optional: Loop limit
    .build()?;
```

## Core Options

### Instructions

The `new()` parameter sets the agent's base instructions:

```rust
ConfigurableAgentBuilder::new(
    "You are a helpful assistant specialized in data analysis."
)
```

### Custom System Prompt

Override the entire system prompt (advanced):

```rust
.with_system_prompt(
    "You are JARVIS. Respond formally and address the user as 'Sir'."
)
```

> **Note**: This replaces the default Deep Agent prompt entirely. Only use when you need complete control.

### Prompt Format

Choose between JSON (default) and TOON (token-efficient):

```rust
use agents_sdk::PromptFormat;

.with_prompt_format(PromptFormat::Toon)  // 30-60% token savings
```

## Model Configuration

### OpenAI

```rust
use agents_sdk::{OpenAiConfig, OpenAiChatModel};

let config = OpenAiConfig::new(api_key, "gpt-4o-mini");
let model = Arc::new(OpenAiChatModel::new(config)?);

.with_model(model)
```

### Anthropic

```rust
use agents_sdk::{AnthropicConfig, AnthropicMessagesModel};

let config = AnthropicConfig::new(api_key, "claude-sonnet-4.5", 4096);
let model = Arc::new(AnthropicMessagesModel::new(config)?);

.with_model(model)
```

### Gemini

```rust
use agents_sdk::{GeminiConfig, GeminiChatModel};

let config = GeminiConfig::new(api_key, "gemini-2.5-pro");
let model = Arc::new(GeminiChatModel::new(config)?);

.with_model(model)
```

## Tools

### Single Tool

```rust
.with_tool(MyTool::as_tool())
```

### Multiple Tools

```rust
.with_tools(vec![
    ToolA::as_tool(),
    ToolB::as_tool(),
    ToolC::as_tool(),
])
```

### Built-in Tools

Enable specific built-in tools:

```rust
// Limit which built-in tools are exposed.
// Built-ins are selected by *tool name* (same names as in LangChain deepagents):
// - write_todos
// - ls, read_file, write_file, edit_file
.with_builtin_tools(["write_todos", "ls", "read_file", "write_file", "edit_file"])
```

## State Persistence

### In-Memory (Development)

```rust
use agents_sdk::persistence::InMemoryCheckpointer;

.with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
```

### Redis (Production)

```rust
use agents_sdk::RedisCheckpointer;

let checkpointer = RedisCheckpointer::new("redis://localhost:6379").await?;
.with_checkpointer(Arc::new(checkpointer))
```

### PostgreSQL (Enterprise)

```rust
use agents_sdk::PostgresCheckpointer;

let checkpointer = PostgresCheckpointer::new(
    "postgresql://user:pass@localhost/agents"
).await?;
.with_checkpointer(Arc::new(checkpointer))
```

### DynamoDB (AWS)

```rust
use agents_sdk::DynamoDbCheckpointer;

let checkpointer = DynamoDbCheckpointer::new("agent-checkpoints").await?;
.with_checkpointer(Arc::new(checkpointer))
```

## Token Tracking

Monitor usage and costs:

```rust
use agents_sdk::{TokenTrackingConfig, TokenCosts};

let config = TokenTrackingConfig {
    enabled: true,
    emit_events: true,
    log_usage: true,
    custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
};

.with_token_tracking_config(config)
```

Or simple enable:

```rust
.with_token_tracking(true)
```

## Human-in-the-Loop (HITL)

Require approval for specific tools using `with_tool_interrupt()`:

```rust
use agents_sdk::HitlPolicy;

// Add one interrupt at a time
.with_tool_interrupt("delete_file", HitlPolicy {
    allow_auto: false,
    note: Some("Deletion requires human approval".to_string()),
})
.with_tool_interrupt("send_email", HitlPolicy {
    allow_auto: false,
    note: Some("Email sending requires review".to_string()),
})
```

## Sub-Agents

Delegate tasks to specialized agents using `with_subagent_config()`:

```rust
use agents_sdk::SubAgentConfig;

let researcher = SubAgentConfig::new(
    "researcher",
    "Searches and analyzes information",
    "You are a research specialist.",
);

let writer = SubAgentConfig::new(
    "writer", 
    "Creates well-written content",
    "You are a content writer.",
);

// Pass an array or vec of SubAgentConfig
.with_subagent_config([researcher, writer])
.with_auto_general_purpose(true)  // Add default general assistant
```

## Summarization

Handle long conversations:

```rust
use agents_sdk::SummarizationConfig;

let summarization = SummarizationConfig {
    messages_to_keep: 50,
    summary_note: "Summarize the key points of this conversation.".to_string(),
};

.with_summarization(summarization)
```

## Event Broadcasting

Receive real-time events:

```rust
use agents_sdk::events::EventDispatcher;

let dispatcher = Arc::new(EventDispatcher::new());
.with_event_dispatcher(dispatcher.clone())

// Listen to events elsewhere
let mut receiver = dispatcher.subscribe();
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        println!("Event: {:?}", event);
    }
});
```

## Security Options

### PII Sanitization

Automatically redact sensitive data (enabled by default):

```rust
.with_pii_sanitization(true)   // Explicit enable
.with_pii_sanitization(false)  // Disable if needed
```

### Prompt Caching

Enable for supported providers:

```rust
.with_prompt_caching(true)
```

## Iteration Limits

Prevent infinite loops:

```rust
.with_max_iterations(15)  // Default is 10
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    TokenTrackingConfig,
    TokenCosts,
    HitlPolicy,
    SubAgentConfig,
    PromptFormat,
    persistence::InMemoryCheckpointer,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(std::env::var("OPENAI_API_KEY")?, "gpt-4o-mini")
    )?);

    let agent = ConfigurableAgentBuilder::new(
        "You are a production-ready assistant with full capabilities."
    )
    // Model
    .with_model(model)
    
    // Tools
    .with_tool(MyTool::as_tool())
    
    // Persistence
    .with_checkpointer(Arc::new(InMemoryCheckpointer::new()))
    
    // Token tracking
    .with_token_tracking_config(TokenTrackingConfig {
        enabled: true,
        emit_events: true,
        log_usage: true,
        custom_costs: Some(TokenCosts::openai_gpt4o_mini()),
    })
    
    // HITL - add one at a time
    .with_tool_interrupt("dangerous_action", HitlPolicy {
        allow_auto: false,
        note: Some("Requires approval".to_string()),
    })
    
    // Sub-agents - pass array or vec to with_subagent_config
    .with_subagent_config([
        SubAgentConfig::new(
            "specialist",
            "Domain expert",
            "You specialize in technical analysis.",
        ),
    ])
    
    // Format and safety
    .with_prompt_format(PromptFormat::Toon)
    .with_pii_sanitization(true)
    .with_max_iterations(15)
    
    .build()?;

    Ok(())
}
```

## Environment Variables

Common environment variables used by the SDK:

| Variable | Description |
|----------|-------------|
| `OPENAI_API_KEY` | OpenAI API key |
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `GOOGLE_API_KEY` | Google Gemini API key |
| `REDIS_URL` | Redis connection string |
| `DATABASE_URL` | PostgreSQL connection string |
| `AWS_REGION` | AWS region for DynamoDB |
| `TAVILY_API_KEY` | Tavily search API key |

