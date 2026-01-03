# In-Memory Checkpointer

Development-focused checkpointer with zero external dependencies.

## Overview

The `InMemoryCheckpointer`:
- Stores state in memory
- No persistence across restarts
- Perfect for development and testing
- Included by default (no feature flag)

## Usage

```rust
use agents_sdk::persistence::InMemoryCheckpointer;
use std::sync::Arc;

let checkpointer = Arc::new(InMemoryCheckpointer::new());

let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
    .with_model(model)
    .with_checkpointer(checkpointer)
    .build()?;
```

## Operations

### Save State

```rust
let thread_id = "user-123";
agent.save_state(thread_id).await?;
```

### Load State

```rust
agent.load_state(thread_id).await?;
```

### Delete State

```rust
checkpointer.delete(&thread_id.into()).await?;
```

## Characteristics

| Property | Value |
|----------|-------|
| Latency | ~0ms |
| Persistence | None (RAM only) |
| Scalability | Single instance |
| Dependencies | None |

## Limitations

- State lost on restart
- Not shared across instances
- Memory grows with state size
- Not suitable for production

## When to Use

✅ **Good for:**
- Local development
- Unit tests
- Quick prototyping
- CI/CD pipelines

❌ **Not for:**
- Production deployments
- Multi-instance setups
- Long-running sessions
- Data persistence requirements

## Testing Example

```rust
#[tokio::test]
async fn test_agent_conversation() {
    let checkpointer = Arc::new(InMemoryCheckpointer::new());
    let agent = create_test_agent(checkpointer.clone());
    
    // Test conversation
    let response1 = agent.handle_message("Hello", state()).await?;
    agent.save_state("test-thread").await?;
    
    // Verify state was saved
    agent.load_state("test-thread").await?;
    let response2 = agent.handle_message("Continue", state()).await?;
    
    // State is available within test
    assert!(response2.state.messages.len() > 2);
}
```

