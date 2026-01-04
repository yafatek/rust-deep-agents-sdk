# Agents

An **Agent** is the central orchestrator in the Deep Agents SDK. It receives user messages, decides which tools to use, and generates responses.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                      DeepAgent                          │
├─────────────────────────────────────────────────────────┤
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────────┐ │
│  │   Planner   │  │  Middleware │  │  Tool Registry  │ │
│  │   (LLM)     │  │    Stack    │  │                 │ │
│  └─────────────┘  └─────────────┘  └─────────────────┘ │
│         │               │                   │          │
│         ▼               ▼                   ▼          │
│  ┌─────────────────────────────────────────────────┐   │
│  │              Message Processing Loop             │   │
│  │  1. Receive message                              │   │
│  │  2. Plan response (LLM call)                     │   │
│  │  3. Execute tools if needed                      │   │
│  │  4. Generate final response                      │   │
│  └─────────────────────────────────────────────────┘   │
└─────────────────────────────────────────────────────────┘
```

## Creating Agents

### Using ConfigurableAgentBuilder (Recommended)

```rust
use agents_sdk::{ConfigurableAgentBuilder, OpenAiConfig, OpenAiChatModel};
use std::sync::Arc;

let model = Arc::new(OpenAiChatModel::new(
    OpenAiConfig::new(api_key, "gpt-4o-mini")
)?);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_tool(MyTool::as_tool())
    .build()?;
```

### Using Factory Functions

```rust
use agents_sdk::{create_deep_agent, get_default_model, CreateDeepAgentParams};

let model = get_default_model()?;
let agent = create_deep_agent(CreateDeepAgentParams {
    instructions: "You are a helpful assistant.".to_string(),
    model: Some(model),
    ..Default::default()
})?;
```

## Agent Lifecycle

### 1. Initialization

When you call `.build()`, the agent:
- Validates configuration
- Initializes the planner (LLM interface)
- Registers all tools
- Sets up middleware pipeline
- Prepares event dispatching

### 2. Message Handling

```rust
let response = agent.handle_message(
    "What's the weather?",
    Arc::new(state)
).await?;
```

The message flows through:

1. **Pre-processing middleware** (PII sanitization, logging)
2. **Planner** (LLM decides action)
3. **Tool execution** (if tools are called)
4. **Post-processing middleware** (token tracking, events)
5. **Response generation**

### 3. Tool Execution Loop

If the LLM decides to use tools:

```
┌──────────────────────────────────────────┐
│           Tool Execution Loop            │
├──────────────────────────────────────────┤
│  while tools_to_call:                    │
│    1. Check HITL policies                │
│    2. Execute tool                       │
│    3. Collect result                     │
│    4. Send result back to LLM            │
│    5. LLM decides: more tools or respond │
└──────────────────────────────────────────┘
```

The loop continues until:
- LLM provides a final response (no tool calls)
- Max iterations reached (default: 10)
- An error occurs

### 4. State Updates

After each message:

```rust
// Response includes updated state
let new_state = response.state;

// Continue conversation with new state
let next_response = agent.handle_message(
    "Follow-up question",
    Arc::new(new_state)
).await?;
```

## Agent Response

The `handle_message` method returns an `AgentResponse`:

```rust
pub struct AgentResponse {
    pub content: MessageContent,
    pub state: AgentStateSnapshot,
    pub tool_calls: Vec<ToolInvocation>,
    pub usage: Option<TokenUsage>,
}
```

### Accessing the Response

```rust
let response = agent.handle_message(msg, state).await?;

// Get text content
let text = response.content.as_text().unwrap_or_default();

// Check what tools were called
for tool_call in &response.tool_calls {
    println!("Called: {} with {:?}", tool_call.name, tool_call.arguments);
}

// Get token usage
if let Some(usage) = &response.usage {
    println!("Tokens: {} input, {} output", usage.prompt_tokens, usage.completion_tokens);
}
```

## Agent Traits

The SDK defines key traits for extensibility:

### AgentHandle

```rust
#[async_trait]
pub trait AgentHandle: Send + Sync {
    async fn handle_message(
        &self,
        message: &str,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentResponse>;
}
```

### PlannerHandle

```rust
#[async_trait]
pub trait PlannerHandle: Send + Sync {
    async fn plan(
        &self,
        messages: &[AgentMessage],
        tools: &[ToolSchema],
    ) -> anyhow::Result<PlannerResponse>;
}
```

## Custom Agents

For advanced use cases, implement custom agents:

```rust
use agents_core::agent::AgentHandle;
use async_trait::async_trait;

struct MyCustomAgent {
    // Your fields
}

#[async_trait]
impl AgentHandle for MyCustomAgent {
    async fn handle_message(
        &self,
        message: &str,
        state: Arc<AgentStateSnapshot>,
    ) -> anyhow::Result<AgentResponse> {
        // Custom logic
    }
}
```

## Best Practices

### 1. Keep Instructions Clear

```rust
// Good: Specific role and capabilities
ConfigurableAgentBuilder::new(
    "You are a customer support agent for TechCorp. 
     You can check order status, process returns, and answer product questions.
     Always be polite and professional."
)

// Avoid: Vague instructions
ConfigurableAgentBuilder::new("You are helpful.")
```

### 2. Use Appropriate Models

| Use Case | Recommended Model |
|----------|-------------------|
| Simple tasks | `gpt-4o-mini`, `claude-haiku-4.5` |
| Complex reasoning | `gpt-4o`, `claude-sonnet-4.5` |
| Research/analysis | `o1-pro`, `claude-opus-4.5` |
| Fast responses | `gemini-2.5-flash` |

### 3. Set Iteration Limits

```rust
.with_max_iterations(15)  // Prevent infinite loops
```

### 4. Enable Monitoring

```rust
.with_token_tracking(true)  // Track costs
```

## Debugging

Enable detailed logging:

```rust
tracing_subscriber::fmt()
    .with_env_filter("agents_runtime=debug,agents_core=debug")
    .init();
```

Or via environment:

```bash
RUST_LOG=agents_runtime=debug cargo run
```

