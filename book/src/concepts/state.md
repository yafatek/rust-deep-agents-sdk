# State Management

State management enables agents to maintain context, remember conversations, and track progress across interactions.

## AgentStateSnapshot

The core state structure:

```rust
pub struct AgentStateSnapshot {
    pub messages: Vec<AgentMessage>,
    pub todos: Vec<TodoItem>,
    pub current_interrupt: Option<HitlInterrupt>,
    pub metadata: HashMap<String, Value>,
}
```

## Creating State

### Default State

```rust
use agents_sdk::state::AgentStateSnapshot;
use std::sync::Arc;

let state = Arc::new(AgentStateSnapshot::default());
```

### With Initial Data

```rust
let mut state = AgentStateSnapshot::default();
state.metadata.insert("user_id".to_string(), json!("user-123"));
state.metadata.insert("session_start".to_string(), json!(chrono::Utc::now().to_rfc3339()));

let state = Arc::new(state);
```

## State Flow

```
┌────────────────────────────────────────────────────────┐
│                   State Flow                           │
├────────────────────────────────────────────────────────┤
│                                                        │
│  Request:                                              │
│  ┌──────────┐    ┌───────────┐    ┌──────────────┐   │
│  │ Message  │ -> │   Agent   │ -> │   Response   │   │
│  │ + State  │    │ Process   │    │ + New State  │   │
│  └──────────┘    └───────────┘    └──────────────┘   │
│                                                        │
│  Persistence:                                          │
│  ┌──────────────┐          ┌──────────────────────┐   │
│  │ save_state() │ -------> │  Checkpointer        │   │
│  │ load_state() │ <------- │  (Redis/PG/DynamoDB) │   │
│  └──────────────┘          └──────────────────────┘   │
│                                                        │
└────────────────────────────────────────────────────────┘
```

## Using State Across Messages

```rust
let mut state = Arc::new(AgentStateSnapshot::default());

// First message
let response1 = agent.handle_message("Hello, I'm Alice", state).await?;
state = Arc::new(response1.state);

// Second message (maintains context)
let response2 = agent.handle_message("What did I just tell you?", state).await?;
state = Arc::new(response2.state);

// Third message
let response3 = agent.handle_message("Great, now help me with something", state).await?;
```

## Checkpointers

Checkpointers persist state across sessions.

### In-Memory (Development)

```rust
use agents_sdk::persistence::InMemoryCheckpointer;

let checkpointer = Arc::new(InMemoryCheckpointer::new());

let agent = ConfigurableAgentBuilder::new("...")
    .with_checkpointer(checkpointer)
    .build()?;
```

### Redis (Production)

```rust
use agents_sdk::RedisCheckpointer;

let checkpointer = Arc::new(
    RedisCheckpointer::new("redis://localhost:6379").await?
);

let agent = ConfigurableAgentBuilder::new("...")
    .with_checkpointer(checkpointer)
    .build()?;
```

### PostgreSQL (Enterprise)

```rust
use agents_sdk::PostgresCheckpointer;

let checkpointer = Arc::new(
    PostgresCheckpointer::new("postgresql://user:pass@localhost/agents").await?
);

let agent = ConfigurableAgentBuilder::new("...")
    .with_checkpointer(checkpointer)
    .build()?;
```

### DynamoDB (AWS)

```rust
use agents_sdk::DynamoDbCheckpointer;

let checkpointer = Arc::new(
    DynamoDbCheckpointer::new("agent-checkpoints").await?
);

let agent = ConfigurableAgentBuilder::new("...")
    .with_checkpointer(checkpointer)
    .build()?;
```

## Saving and Loading State

```rust
// Thread ID identifies the conversation
let thread_id = "user-123-session-456";

// Process message
let response = agent.handle_message("Hello!", state).await?;

// Save state to storage
agent.save_state(thread_id).await?;

// Later: Load state to resume conversation
agent.load_state(thread_id).await?;

// Continue conversation
let response = agent.handle_message("Continue from where we left off", state).await?;
```

## Todo Items

Agents can track tasks:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TodoItem {
    pub id: String,
    pub content: String,
    pub status: TodoStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TodoStatus {
    Pending,
    InProgress,
    Completed,
    Cancelled,
}
```

Accessing todos:

```rust
for todo in &state.todos {
    println!("[{:?}] {}: {}", todo.status, todo.id, todo.content);
}
```

## Metadata

Store custom data in state:

```rust
// Set metadata
state.metadata.insert("customer_tier".to_string(), json!("premium"));
state.metadata.insert("locale".to_string(), json!("en-US"));

// Read metadata
if let Some(tier) = state.metadata.get("customer_tier") {
    println!("Customer tier: {}", tier);
}
```

## HITL Interrupts

State tracks pending human approvals:

```rust
if let Some(interrupt) = &state.current_interrupt {
    println!("Pending approval for tool: {}", interrupt.tool_name);
    println!("Arguments: {:?}", interrupt.tool_args);
    
    // Handle approval/rejection
    match get_human_decision() {
        Decision::Approve => agent.resume_with_approval(HitlAction::Accept).await?,
        Decision::Reject => agent.resume_with_approval(HitlAction::Reject).await?,
    }
}
```

## State Best Practices

### 1. Use Unique Thread IDs

```rust
// Good: Unique per user/session
let thread_id = format!("user-{}-session-{}", user_id, session_id);

// Bad: Generic IDs
let thread_id = "default";
```

### 2. Clean Up Old State

```rust
// Implement TTL in your checkpointer
// Or periodically clean up old sessions
async fn cleanup_old_sessions(days: u32) {
    // Implementation
}
```

### 3. Handle State Errors

```rust
match agent.load_state(thread_id).await {
    Ok(_) => println!("Resumed conversation"),
    Err(e) => {
        tracing::warn!("Failed to load state: {}, starting fresh", e);
        // Continue with default state
    }
}
```

### 4. Don't Store Sensitive Data

```rust
// Good: Store references
state.metadata.insert("user_id".to_string(), json!("user-123"));

// Bad: Store sensitive data directly
state.metadata.insert("credit_card".to_string(), json!("4111...")); // Don't do this!
```

## Custom Checkpointer

Implement your own storage backend:

```rust
use agents_core::persistence::{Checkpointer, ThreadId};
use async_trait::async_trait;

struct MyCheckpointer {
    // Your storage client
}

#[async_trait]
impl Checkpointer for MyCheckpointer {
    async fn save(&self, thread_id: &ThreadId, state: &AgentStateSnapshot) -> anyhow::Result<()> {
        // Serialize and store
        Ok(())
    }

    async fn load(&self, thread_id: &ThreadId) -> anyhow::Result<Option<AgentStateSnapshot>> {
        // Fetch and deserialize
        Ok(None)
    }

    async fn delete(&self, thread_id: &ThreadId) -> anyhow::Result<()> {
        // Remove from storage
        Ok(())
    }
}
```

