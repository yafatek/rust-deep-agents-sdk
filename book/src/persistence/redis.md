# Redis Checkpointer

Production-ready checkpointer with low-latency persistence.

## Overview

The `RedisCheckpointer`:
- Sub-millisecond latency
- Persistent storage
- Cluster support
- TTL for automatic cleanup

## Installation

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["redis"] }
```

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, RedisCheckpointer};
use std::sync::Arc;

let checkpointer = Arc::new(
    RedisCheckpointer::new("redis://127.0.0.1:6379").await?
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;
```

## Connection Options

### Basic Connection

```rust
RedisCheckpointer::new("redis://localhost:6379").await?
```

### With Authentication

```rust
RedisCheckpointer::new("redis://user:password@localhost:6379").await?
```

### With Database Selection

```rust
RedisCheckpointer::new("redis://localhost:6379/1").await?  // DB 1
```

### With TLS

```rust
RedisCheckpointer::new("rediss://localhost:6379").await?
```

### Environment Variable

```bash
export REDIS_URL="redis://localhost:6379"
```

```rust
let url = std::env::var("REDIS_URL")?;
RedisCheckpointer::new(&url).await?
```

## Configuration

### With TTL

```rust
use std::time::Duration;

let checkpointer = RedisCheckpointer::new("redis://localhost:6379")
    .await?
    .with_ttl(Duration::from_secs(86400 * 7));  // 7 days
```

### With Key Prefix

```rust
let checkpointer = RedisCheckpointer::new("redis://localhost:6379")
    .await?
    .with_prefix("myapp:agents:");
```

## Key Structure

States are stored with keys:
```
{prefix}state:{thread_id}
```

Example:
```
agents:state:user-123-session-456
```

## Operations

### Save State

```rust
agent.save_state("user-123").await?;
// Stored at: agents:state:user-123
```

### Load State

```rust
agent.load_state("user-123").await?;
```

### Delete State

```rust
checkpointer.delete(&"user-123".into()).await?;
```

## Cluster Support

```rust
// Redis Cluster
RedisCheckpointer::new("redis://node1:6379,node2:6379,node3:6379").await?
```

## Characteristics

| Property | Value |
|----------|-------|
| Latency | ~1ms |
| Persistence | Disk (configurable) |
| Scalability | Cluster/Sentinel |
| Dependencies | Redis server |

## Best Practices

### 1. Use Connection Pooling

The checkpointer manages connections internally.

### 2. Set Appropriate TTL

```rust
// 24 hours for short sessions
.with_ttl(Duration::from_secs(86400))

// 30 days for long-term users
.with_ttl(Duration::from_secs(86400 * 30))
```

### 3. Use Meaningful Prefixes

```rust
// Separate by environment
.with_prefix("prod:agents:")
.with_prefix("staging:agents:")
```

### 4. Monitor Memory

```bash
redis-cli INFO memory
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    RedisCheckpointer,
    state::AgentStateSnapshot,
};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let redis_url = std::env::var("REDIS_URL")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());

    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let checkpointer = Arc::new(
        RedisCheckpointer::new(&redis_url)
            .await?
            .with_prefix("myapp:")
            .with_ttl(Duration::from_secs(86400 * 7))
    );

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_checkpointer(checkpointer)
        .build()?;

    let thread_id = "user-123";

    // Try to resume existing conversation
    if let Err(_) = agent.load_state(thread_id).await {
        println!("Starting new conversation");
    } else {
        println!("Resuming existing conversation");
    }

    let response = agent.handle_message(
        "Hello, remember me?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("Agent: {}", response.content.as_text().unwrap_or_default());

    // Save for next time
    agent.save_state(thread_id).await?;

    Ok(())
}
```

