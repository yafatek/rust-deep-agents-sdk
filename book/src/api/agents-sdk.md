# agents-sdk API Reference

The unified SDK re-exports all commonly used types.

## Core Re-exports

### Agent Types

```rust
pub use agents_core::agent::{AgentHandle, AgentStream};
```

### LLM Types

```rust
pub use agents_core::llm::{ChunkStream, StreamChunk};
```

### Tool Types

```rust
pub use agents_core::tools::{
    Tool, ToolBox, ToolContext, ToolParameterSchema, 
    ToolRegistry, ToolResult, ToolSchema,
};
```

### Modules

```rust
pub use agents_core::{
    agent, events, hitl, llm, messaging, 
    persistence, security, state, tools
};
```

## Runtime Re-exports

### Builders and Agents

```rust
pub use agents_runtime::{
    create_async_deep_agent,
    create_deep_agent,
    get_default_model,
    ConfigurableAgentBuilder,
    DeepAgent,
};
```

### Provider Configurations

```rust
pub use agents_runtime::{
    AnthropicConfig,
    AnthropicMessagesModel,
    GeminiChatModel,
    GeminiConfig,
    OpenAiChatModel,
    OpenAiConfig,
};
```

### Configuration Types

```rust
pub use agents_runtime::{
    HitlPolicy,
    SubAgentConfig,
    SummarizationConfig,
};
```

## Token Tracking

```rust
pub use agents_core::events::TokenUsage;
pub use agents_runtime::middleware::token_tracking::{
    TokenCosts, TokenTrackingConfig, 
    TokenTrackingMiddleware, TokenUsageSummary,
};
```

## Feature-Gated Re-exports

### Toolkit (`toolkit` feature)

```rust
#[cfg(feature = "toolkit")]
pub use agents_toolkit::*;
pub use agents_macros::tool;
```

### AWS (`aws` feature)

```rust
#[cfg(feature = "aws")]
pub use agents_aws::*;
```

### Persistence

```rust
#[cfg(feature = "redis")]
pub use agents_persistence::RedisCheckpointer;

#[cfg(feature = "postgres")]
pub use agents_persistence::PostgresCheckpointer;
```

## Prelude

Import common types quickly:

```rust
use agents_sdk::prelude::*;
```

Includes:
- `AgentHandle`, `PlannerHandle`
- `AgentMessage`, `MessageContent`, `MessageRole`
- `Checkpointer`, `ThreadId`
- `AgentStateSnapshot`
- `ConfigurableAgentBuilder`, `get_default_model`

## Feature Flags

| Flag | Description |
|------|-------------|
| `toolkit` | Built-in tools and `#[tool]` macro (default) |
| `toon` | TOON format support |
| `redis` | Redis checkpointer |
| `postgres` | PostgreSQL checkpointer |
| `dynamodb` | DynamoDB checkpointer |
| `aws` | AWS integrations |
| `persistence` | Redis + PostgreSQL |
| `aws-full` | AWS + DynamoDB |
| `full` | Everything |

