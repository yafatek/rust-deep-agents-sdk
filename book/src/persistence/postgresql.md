# PostgreSQL Checkpointer

Enterprise-grade checkpointer with queryable state storage.

## Overview

The `PostgresCheckpointer`:
- ACID-compliant storage
- Complex queries on state
- Built-in analytics capability
- Long-term data retention

## Installation

```toml
[dependencies]
agents-sdk = { version = "0.0.29", features = ["postgres"] }
```

## Quick Start

```rust
use agents_sdk::{ConfigurableAgentBuilder, PostgresCheckpointer};
use std::sync::Arc;

let checkpointer = Arc::new(
    PostgresCheckpointer::new("postgresql://user:pass@localhost/agents").await?
);

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;
```

## Connection Options

### Basic Connection

```rust
PostgresCheckpointer::new("postgresql://localhost/agents").await?
```

### With Credentials

```rust
PostgresCheckpointer::new("postgresql://user:password@localhost/agents").await?
```

### With SSL

```rust
PostgresCheckpointer::new("postgresql://localhost/agents?sslmode=require").await?
```

### Environment Variable

```bash
export DATABASE_URL="postgresql://user:pass@localhost/agents"
```

```rust
let url = std::env::var("DATABASE_URL")?;
PostgresCheckpointer::new(&url).await?
```

## Schema

The checkpointer creates this table automatically:

```sql
CREATE TABLE IF NOT EXISTS agent_states (
    thread_id VARCHAR(255) PRIMARY KEY,
    state JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_agent_states_updated_at ON agent_states(updated_at);
```

## Operations

### Save State

```rust
agent.save_state("user-123").await?;
```

### Load State

```rust
agent.load_state("user-123").await?;
```

### Delete State

```rust
checkpointer.delete(&"user-123".into()).await?;
```

## Querying State

Direct database queries for analytics:

```sql
-- Recent conversations
SELECT thread_id, updated_at 
FROM agent_states 
ORDER BY updated_at DESC 
LIMIT 10;

-- Message count per thread
SELECT 
    thread_id,
    jsonb_array_length(state->'messages') as message_count
FROM agent_states;

-- Find conversations mentioning a topic
SELECT thread_id, state
FROM agent_states
WHERE state::text ILIKE '%rust%';

-- Active threads in last 24h
SELECT COUNT(*) 
FROM agent_states 
WHERE updated_at > NOW() - INTERVAL '24 hours';
```

## Configuration

### Connection Pool

```rust
let checkpointer = PostgresCheckpointer::new(&url)
    .await?
    .with_pool_size(10);  // Max connections
```

### Custom Table Name

```rust
let checkpointer = PostgresCheckpointer::new(&url)
    .await?
    .with_table_name("custom_agent_states");
```

## Characteristics

| Property | Value |
|----------|-------|
| Latency | ~5ms |
| Persistence | Durable |
| Scalability | Replicas/Sharding |
| Dependencies | PostgreSQL |

## Migrations

Run on deployment:

```sql
-- Add index for customer lookups
CREATE INDEX idx_agent_states_customer 
ON agent_states ((state->>'customer_id'));

-- Add TTL column
ALTER TABLE agent_states 
ADD COLUMN expires_at TIMESTAMPTZ;

-- Cleanup job
DELETE FROM agent_states 
WHERE expires_at < NOW();
```

## Best Practices

### 1. Use Connection Pooling

```rust
// Pool managed internally, configure size based on load
.with_pool_size(20)
```

### 2. Index for Your Queries

```sql
-- If querying by customer
CREATE INDEX ON agent_states ((state->>'customer_id'));

-- If querying by date range
CREATE INDEX ON agent_states (updated_at);
```

### 3. Implement Cleanup

```sql
-- Scheduled job to remove old states
DELETE FROM agent_states 
WHERE updated_at < NOW() - INTERVAL '90 days';
```

### 4. Monitor Size

```sql
SELECT pg_size_pretty(pg_total_relation_size('agent_states'));
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    PostgresCheckpointer,
    state::AgentStateSnapshot,
};
use std::sync::Arc;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let db_url = std::env::var("DATABASE_URL")?;

    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let checkpointer = Arc::new(
        PostgresCheckpointer::new(&db_url).await?
    );

    let agent = ConfigurableAgentBuilder::new(
        "You are a customer support agent."
    )
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;

    // Customer conversation
    let thread_id = "customer-456-ticket-789";
    
    // Try to resume
    agent.load_state(thread_id).await.ok();

    let response = agent.handle_message(
        "What's the status of my order?",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    println!("{}", response.content.as_text().unwrap_or_default());

    // Persist for support team
    agent.save_state(thread_id).await?;

    Ok(())
}
```

