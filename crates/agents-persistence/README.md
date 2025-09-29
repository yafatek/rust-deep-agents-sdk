# agents-persistence

Database-backed persistence implementations for the Rust Deep Agents SDK.

This crate provides production-ready checkpointer implementations for various storage backends, allowing users to choose the persistence layer that best fits their infrastructure.

## Available Backends

| Backend | Feature Flag | Best For |
|---------|-------------|----------|
| **Redis** | `redis` | High-performance, distributed systems, caching |
| **PostgreSQL** | `postgres` | ACID guarantees, relational data, analytics |
| **DynamoDB** | (in `agents-aws` crate) | AWS serverless, auto-scaling, global tables |

## Installation

Add to your `Cargo.toml`:

```toml
# Choose the backend(s) you need
[dependencies]
agents-sdk = { version = "0.0.1", features = ["redis"] }
# or
agents-sdk = { version = "0.0.1", features = ["postgres"] }
# or
agents-sdk = { version = "0.0.1", features = ["dynamodb"] }

# Enable multiple backends
agents-sdk = { version = "0.0.1", features = ["redis", "postgres"] }
```

## Quick Start

### Redis Checkpointer

```rust
use agents_sdk::{RedisCheckpointer, ConfigurableAgentBuilder};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Basic usage
    let checkpointer = Arc::new(
        RedisCheckpointer::new("redis://127.0.0.1:6379").await?
    );

    // With custom configuration
    let checkpointer = Arc::new(
        RedisCheckpointer::builder()
            .url("redis://127.0.0.1:6379")
            .namespace("myapp")
            .ttl(Duration::from_secs(86400)) // 24 hours
            .build()
            .await?
    );

    // Use with agent
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
        .with_checkpointer(checkpointer)
        .build()?;

    // Save and load state
    agent.save_state("user-123").await?;
    agent.load_state("user-123").await?;

    Ok(())
}
```

### PostgreSQL Checkpointer

```rust
use agents_sdk::{PostgresCheckpointer, ConfigurableAgentBuilder};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Basic usage
    let checkpointer = Arc::new(
        PostgresCheckpointer::new("postgresql://user:pass@localhost/agents").await?
    );

    // With custom configuration
    let checkpointer = Arc::new(
        PostgresCheckpointer::builder()
            .url("postgresql://user:pass@localhost/agents")
            .table_name("my_checkpoints")
            .max_connections(20)
            .build()
            .await?
    );

    // Use with agent
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
        .with_checkpointer(checkpointer)
        .build()?;

    Ok(())
}
```

### DynamoDB Checkpointer (AWS)

```rust
use agents_sdk::{DynamoDbCheckpointer, ConfigurableAgentBuilder};
use std::sync::Arc;
use std::time::Duration;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Basic usage
    let checkpointer = Arc::new(
        DynamoDbCheckpointer::new("agent-checkpoints").await?
    );

    // With custom configuration
    let checkpointer = Arc::new(
        DynamoDbCheckpointer::builder()
            .table_name("my-agents")
            .ttl(Duration::from_secs(86400 * 7)) // 7 days
            .build()
            .await?
    );

    // Use with agent
    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant")
        .with_checkpointer(checkpointer)
        .build()?;

    Ok(())
}
```

## Setup Instructions

### Redis

**Using Docker:**
```bash
docker run -d -p 6379:6379 redis:7-alpine
```

**Production Setup:**
- Use Redis Cluster for high availability
- Enable AOF persistence for durability
- Configure maxmemory-policy for eviction

### PostgreSQL

**Using Docker:**
```bash
docker run -d \
  -e POSTGRES_DB=agents \
  -e POSTGRES_USER=user \
  -e POSTGRES_PASSWORD=pass \
  -p 5432:5432 \
  postgres:16-alpine
```

**Table Creation:**
The checkpointer automatically creates the required table:
```sql
CREATE TABLE IF NOT EXISTS agent_checkpoints (
    thread_id TEXT PRIMARY KEY,
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

### DynamoDB

**Using AWS CLI:**
```bash
aws dynamodb create-table \
  --table-name agent-checkpoints \
  --attribute-definitions AttributeName=thread_id,AttributeType=S \
  --key-schema AttributeName=thread_id,KeyType=HASH \
  --billing-mode PAY_PER_REQUEST
```

**Enable TTL (optional):**
```bash
aws dynamodb update-time-to-live \
  --table-name agent-checkpoints \
  --time-to-live-specification "Enabled=true, AttributeName=ttl"
```

## Feature Comparison

| Feature | Redis | PostgreSQL | DynamoDB |
|---------|-------|------------|----------|
| **Performance** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| **Durability** | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Scalability** | ⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐⭐⭐⭐ |
| **Cost** | $ | $$ | $ (pay-per-request) |
| **Setup Complexity** | Low | Medium | Low (managed) |
| **Query Capabilities** | Limited | Full SQL | Limited |
| **Multi-region** | Clustering | Replication | Global Tables |

## Best Practices

### Redis
- Use connection pooling (built-in)
- Set appropriate TTL for automatic cleanup
- Use namespaces for multi-tenant applications
- Monitor memory usage
- Enable persistence (AOF/RDB) for durability

### PostgreSQL
- Use connection pooling (built-in via sqlx)
- Create indexes on `updated_at` for efficient queries
- Regular backups and point-in-time recovery
- Monitor connection limits
- Use read replicas for high-read workloads

### DynamoDB
- Use on-demand billing for variable workloads
- Enable TTL for automatic cleanup
- Use global tables for multi-region deployments
- Monitor read/write capacity
- Use DynamoDB Streams for event-driven architectures

## Error Handling

All checkpointers return `anyhow::Result` and provide detailed error messages:

```rust
match checkpointer.save_state(&thread_id, &state).await {
    Ok(()) => println!("State saved successfully"),
    Err(e) => eprintln!("Failed to save state: {}", e),
}
```

## Testing

Each backend includes integration tests:

```bash
# Requires running services
cargo test --features redis --package agents-persistence -- --ignored
cargo test --features postgres --package agents-persistence -- --ignored
cargo test --features dynamodb --package agents-aws -- --ignored
```

## License

MIT OR Apache-2.0

