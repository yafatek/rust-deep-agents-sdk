# Events API Reference

Complete reference for the event system.

## AgentEvent Enum

```rust
pub enum AgentEvent {
    AgentStarted(AgentStartedEvent),
    AgentCompleted(AgentCompletedEvent),
    ToolStarted(ToolStartedEvent),
    ToolCompleted(ToolCompletedEvent),
    ToolFailed(ToolFailedEvent),
    SubAgentStarted(SubAgentStartedEvent),
    SubAgentCompleted(SubAgentCompletedEvent),
    TodosUpdated(TodosUpdatedEvent),
    StateCheckpointed(StateCheckpointedEvent),
    PlanningComplete(PlanningCompleteEvent),
    TokenUsage(TokenUsageEvent),
    StreamingToken(StreamingTokenEvent),
}
```

## Event Metadata

All events include:

```rust
pub struct EventMetadata {
    pub thread_id: String,
    pub correlation_id: String,
    pub customer_id: Option<String>,
    pub timestamp: String,  // RFC3339
}
```

## Event Types

### AgentStartedEvent

```rust
pub struct AgentStartedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub message_preview: String,  // Truncated
}
```

### AgentCompletedEvent

```rust
pub struct AgentCompletedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub duration_ms: u64,
    pub response_preview: String,
    pub response: String,
}
```

### ToolStartedEvent

```rust
pub struct ToolStartedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub args_preview: String,
}
```

### ToolCompletedEvent

```rust
pub struct ToolCompletedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub duration_ms: u64,
    pub result_preview: String,
}
```

### ToolFailedEvent

```rust
pub struct ToolFailedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub error: String,
}
```

### TokenUsageEvent

```rust
pub struct TokenUsageEvent {
    pub metadata: EventMetadata,
    pub usage: TokenUsage,
}

pub struct TokenUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
    pub estimated_cost: Option<f64>,
}
```

### StreamingTokenEvent

```rust
pub struct StreamingTokenEvent {
    pub metadata: EventMetadata,
    pub content: String,
    pub index: u32,
}
```

## EventDispatcher

### Creation

```rust
let dispatcher = Arc::new(EventDispatcher::new());
```

### Subscription

```rust
let mut receiver = dispatcher.subscribe();

while let Ok(event) = receiver.recv().await {
    // Handle event
}
```

### With Builder

```rust
let agent = ConfigurableAgentBuilder::new("...")
    .with_event_dispatcher(dispatcher)
    .build()?;
```

## EventBroadcaster Trait

```rust
#[async_trait]
pub trait EventBroadcaster: Send + Sync {
    fn id(&self) -> &str;
    fn supports_streaming(&self) -> bool;
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()>;
}
```

### Implementation Example

```rust
struct MyBroadcaster;

#[async_trait]
impl EventBroadcaster for MyBroadcaster {
    fn id(&self) -> &str { "my_broadcaster" }
    fn supports_streaming(&self) -> bool { true }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        println!("{:?}", event);
        Ok(())
    }
}
```

## Methods

### event_type_name

```rust
impl AgentEvent {
    pub fn event_type_name(&self) -> &'static str
}
```

Returns: `"agent_started"`, `"tool_completed"`, etc.

### metadata

```rust
impl AgentEvent {
    pub fn metadata(&self) -> &EventMetadata
}
```

Returns the event's metadata.

## Serialization

Events are `Serialize` + `Deserialize`:

```rust
let json = serde_json::to_string(&event)?;
let event: AgentEvent = serde_json::from_str(&json)?;
```

## Pattern Matching

```rust
match event {
    AgentEvent::AgentStarted(e) => { /* ... */ }
    AgentEvent::ToolCompleted(e) => { /* ... */ }
    AgentEvent::TokenUsage(e) => { /* ... */ }
    AgentEvent::StreamingToken(e) => { /* ... */ }
    _ => {}
}
```

