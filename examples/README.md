# Deep Agents SDK Examples

This directory contains examples demonstrating how to use the Rust Deep Agents SDK. The API closely mirrors the Python SDK for familiar usage patterns.

## Examples Overview

### 1. Simple Agent (`simple-agent/`)
Basic usage example showing how to create a Deep Agent with custom tools.

**Python equivalent:**
```python
from deepagents import create_deep_agent

agent = create_deep_agent(
    tools=[internet_search],
    instructions="You are an expert researcher...",
)

result = agent.invoke({"messages": [{"role": "user", "content": "what is langgraph?"}]})
```

**Rust equivalent:**
```rust
use agents_runtime::{create_deep_agent, CreateDeepAgentParams};

let agent = create_deep_agent(CreateDeepAgentParams {
    tools: vec![Arc::new(InternetSearchTool)],
    instructions: "You are an expert researcher...".to_string(),
    ..Default::default()
})?;

let response = agent.handle_message("what is langgraph?", state).await?;
```

### 2. Builder Example (`builder-example/`)
Advanced usage with the fluent builder API, subagents, and HITL.

**Python equivalent:**
```python
from deepagents import create_deep_agent
from langgraph.checkpoint.memory import InMemorySaver

agent = create_deep_agent(
    tools=[calculator, research],
    instructions="You are a helpful research assistant...",
    model=model,
    subagents=[math_subagent, research_subagent],
    tool_configs={"calculator": True},
    checkpointer=InMemorySaver()
)
```

**Rust equivalent:**
```rust
use agents_runtime::ConfigurableAgentBuilder;

let agent = ConfigurableAgentBuilder::new("You are a helpful research assistant...")
    .with_model(get_default_model()?)
    .with_tools([calculator, research])
    .with_subagent_config([math_subagent, research_subagent])
    .with_tool_interrupt("calculator", HitlPolicy { allow_auto: false, note: Some("Requires approval".to_string()) })
    .with_checkpointer(checkpointer)
    .build()?;
```

### 3. Research Agent (`research-agent/`)
Complex research agent with file operations and specialized subagents.

## API Comparison

| Feature | Python SDK | Rust SDK |
|---------|------------|----------|
| **Basic Creation** | `create_deep_agent(tools, instructions)` | `create_deep_agent(CreateDeepAgentParams { tools, instructions, .. })` |
| **Builder Pattern** | N/A | `ConfigurableAgentBuilder::new(instructions).with_*().build()` |
| **Model Selection** | `model=ChatAnthropic(...)` | `.with_model(Arc::new(AnthropicMessagesModel::new(...)))` |
| **Subagents** | `subagents=[{name, description, prompt, tools}]` | `.with_subagent_config([SubAgentConfig { name, description, instructions, tools }])` |
| **HITL** | `tool_configs={"tool": True}` | `.with_tool_interrupt("tool", HitlPolicy { allow_auto: false, .. })` |
| **State Persistence** | `checkpointer=InMemorySaver()` | `.with_checkpointer(Arc::new(InMemoryCheckpointer::new()))` |
| **Message Handling** | `agent.invoke({"messages": [...]})` | `agent.handle_message("text", state).await` |

## Key Differences

### 1. **Type Safety**
- **Python**: Dynamic typing, runtime errors possible
- **Rust**: Compile-time type checking, memory safety guarantees

### 2. **Async Handling**
- **Python**: `async_create_deep_agent()` for async tools
- **Rust**: All operations are async by default with `tokio`

### 3. **Memory Management**
- **Python**: Garbage collected
- **Rust**: Zero-cost abstractions with `Arc<dyn Trait>` for shared ownership

### 4. **Error Handling**
- **Python**: Exceptions
- **Rust**: `Result<T, E>` types with `anyhow` for error chaining

## Running Examples

```bash
# Simple agent
cd examples/simple-agent
cargo run

# Builder example  
cd examples/builder-example
cargo run

# Research agent (requires API keys)
cd examples/research-agent
export OPENAI_API_KEY="your-key"
cargo run
```

## Environment Setup

Most examples require API keys:

```bash
# For Anthropic (default model)
export ANTHROPIC_API_KEY="your-anthropic-key"

# For OpenAI (alternative)
export OPENAI_API_KEY="your-openai-key"

# For Gemini (alternative)
export GEMINI_API_KEY="your-gemini-key"
```

## Built-in Tools

All agents automatically have access to these built-in tools (unless filtered with `with_builtin_tools()`):

- `write_todos` - Task planning and management
- `ls` - List files in virtual filesystem  
- `read_file` - Read from virtual filesystem
- `write_file` - Write to virtual filesystem
- `edit_file` - Edit files in virtual filesystem
- `task` - Delegate to subagents (when subagents are configured)

## Middleware Stack

The Rust SDK assembles middleware in the same order as Python:

1. **Base System Prompt** - Core instructions
2. **Planning Middleware** - Todo management (`write_todos` tool)
3. **Filesystem Middleware** - File operations (`ls`, `read_file`, `write_file`, `edit_file`)
4. **SubAgent Middleware** - Task delegation (`task` tool)
5. **Summarization Middleware** - Context window management (optional)
6. **Anthropic Prompt Caching** - Performance optimization (optional)
7. **Human-in-the-Loop** - Tool approval workflows (optional)

This ensures identical behavior between Python and Rust implementations.
