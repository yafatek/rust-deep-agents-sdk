# Persistence Overview

State persistence enables agents to maintain context across sessions.

## Why Persistence?

- **Conversation continuity** - Resume conversations after restarts
- **Multi-session users** - Track long-running interactions
- **Scalability** - Share state across instances
- **Recovery** - Restore state after failures

## Available Backends

| Backend | Best For | Feature Flag |
|---------|----------|--------------|
| In-Memory | Development, testing | Default |
| Redis | Production, low-latency | `redis` |
| PostgreSQL | Enterprise, analytics | `postgres` |
| DynamoDB | AWS-native, serverless | `dynamodb` |

## Quick Comparison

```
┌──────────────┬─────────┬────────┬───────────────┐
│   Backend    │ Latency │ Scale  │   Best Use    │
├──────────────┼─────────┼────────┼───────────────┤
│ In-Memory    │ ~0ms    │ Single │ Development   │
│ Redis        │ ~1ms    │ Multi  │ Production    │
│ PostgreSQL   │ ~5ms    │ Multi  │ Enterprise    │
│ DynamoDB     │ ~10ms   │ Global │ AWS/Serverless│
└──────────────┴─────────┴────────┴───────────────┘
```

## Basic Usage

```rust
use agents_sdk::{ConfigurableAgentBuilder, persistence::InMemoryCheckpointer};
use std::sync::Arc;

let checkpointer = Arc::new(InMemoryCheckpointer::new());

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;

// Save state
agent.save_state("user-123").await?;

// Load state
agent.load_state("user-123").await?;
```

## Thread IDs

Thread IDs identify conversation sessions:

```rust
// Good: Unique, meaningful IDs
let thread_id = format!("user-{}-session-{}", user_id, session_id);

// For single user per session
let thread_id = format!("user-{}", user_id);

// For anonymous users
let thread_id = format!("anon-{}", uuid::Uuid::new_v4());
```

## State Structure

```rust
pub struct AgentStateSnapshot {
    pub messages: Vec<AgentMessage>,      // Conversation history
    pub todos: Vec<TodoItem>,             // Task tracking
    pub current_interrupt: Option<HitlInterrupt>, // Pending approvals
    pub metadata: HashMap<String, Value>, // Custom data
}
```

## Checkpointer Trait

All backends implement:

```rust
#[async_trait]
pub trait Checkpointer: Send + Sync {
    async fn save(&self, thread_id: &ThreadId, state: &AgentStateSnapshot) 
        -> anyhow::Result<()>;
    
    async fn load(&self, thread_id: &ThreadId) 
        -> anyhow::Result<Option<AgentStateSnapshot>>;
    
    async fn delete(&self, thread_id: &ThreadId) 
        -> anyhow::Result<()>;
}
```

## Choosing a Backend

### Development

```rust
// Fast, no setup required
Arc::new(InMemoryCheckpointer::new())
```

### Production (Single Region)

```rust
// Redis for low latency
Arc::new(RedisCheckpointer::new("redis://localhost:6379").await?)
```

### Enterprise (Analytics/Compliance)

```rust
// PostgreSQL for queryable data
Arc::new(PostgresCheckpointer::new("postgresql://...").await?)
```

### AWS Serverless

```rust
// DynamoDB for global scale
Arc::new(DynamoDbCheckpointer::new("table-name").await?)
```

## Next Steps

- [In-Memory](./in-memory.md) - Development checkpointer
- [Redis](./redis.md) - Production checkpointer
- [PostgreSQL](./postgresql.md) - Enterprise checkpointer
- [DynamoDB](./dynamodb.md) - AWS checkpointer

