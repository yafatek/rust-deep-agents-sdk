# Event System Documentation

## Overview

The Deep Agents SDK includes a powerful event system that allows you to broadcast agent lifecycle events to multiple channels simultaneously. This enables real-time progress tracking, monitoring, and integration with external systems like WhatsApp, SSE streams, and databases.

## Architecture

The event system follows a publisher-subscriber pattern with these key components:

- **AgentEvent**: Enum representing all possible agent lifecycle events
- **EventBroadcaster**: Trait for implementing custom event handlers
- **EventDispatcher**: Manages multiple broadcasters and dispatches events
- **DeepAgent Integration**: Automatically emits events during agent execution

```
┌─────────────────────────────────────────────────────────────┐
│                      Agent Runtime                          │
│  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │   Planner    │───▶│  Middleware  │───▶│    Tools     │ │
│  └──────────────┘    └──────────────┘    └──────────────┘ │
│         │                    │                    │         │
│         └────────────────────┴────────────────────┘         │
│                              │                               │
│                              ▼                               │
│                    ┌──────────────────┐                     │
│                    │  Event Emitter   │                     │
│                    └──────────────────┘                     │
└─────────────────────────────│───────────────────────────────┘
                              │
                              ▼
                    ┌──────────────────┐
                    │ Event Dispatcher │
                    └──────────────────┘
                              │
                ┌─────────────┼─────────────┐
                │             │             │
                ▼             ▼             ▼
        ┌──────────┐  ┌──────────┐  ┌──────────┐
        │ WhatsApp │  │   SSE    │  │ DynamoDB │
        │Broadcaster│  │Broadcaster│  │Broadcaster│
        └──────────┘  └──────────┘  └──────────┘
```

## Event Types

### AgentEvent Enum

All events are variants of the `AgentEvent` enum:

```rust
use agents_core::events::AgentEvent;

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
}
```

### Event Metadata

All events include common metadata:

```rust
pub struct EventMetadata {
    pub thread_id: String,
    pub correlation_id: String,
    pub customer_id: Option<String>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
```

### When Events Are Emitted

| Event | Trigger Point | Use Case |
|-------|--------------|----------|
| `AgentStarted` | Beginning of `handle_message_internal()` | Track agent invocations |
| `PlanningComplete` | After planner decides next action | Monitor decision-making |
| `ToolStarted` | Before tool execution | Show progress to users |
| `ToolCompleted` | After successful tool execution | Update UI with results |
| `ToolFailed` | After tool execution error | Error handling and retry logic |
| `SubAgentStarted` | Before delegating to sub-agent | Track delegation depth |
| `SubAgentCompleted` | After sub-agent returns | Measure sub-agent performance |
| `TodosUpdated` | After todo list changes | Show task progress |
| `StateCheckpointed` | After successful state save | Monitor persistence |

## EventBroadcaster Trait

Implement this trait to create custom event handlers:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;

#[async_trait]
pub trait EventBroadcaster: Send + Sync {
    /// Unique identifier for this broadcaster
    fn id(&self) -> &str;
    
    /// Broadcast an event (non-blocking)
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()>;
    
    /// Optional: Filter events this broadcaster cares about
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        true // Default: broadcast all events
    }
}
```

### Key Design Principles

1. **Non-Blocking**: Broadcasters run asynchronously and never block agent execution
2. **Error Isolation**: Broadcaster failures are logged but don't affect the agent
3. **Selective Broadcasting**: Use `should_broadcast()` to filter relevant events
4. **Unique IDs**: Each broadcaster has a unique identifier for logging

## Built-in Broadcaster Examples

### 1. Console Logger Broadcaster

Simple broadcaster that logs events to console:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;

pub struct ConsoleLoggerBroadcaster;

#[async_trait]
impl EventBroadcaster for ConsoleLoggerBroadcaster {
    fn id(&self) -> &str {
        "console_logger"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::AgentStarted(e) => {
                println!("🚀 Agent started: {}", e.agent_name);
            }
            AgentEvent::ToolStarted(e) => {
                println!("🔧 Tool started: {}", e.tool_name);
            }
            AgentEvent::ToolCompleted(e) => {
                println!("✅ Tool completed: {} ({}ms)", 
                    e.tool_name, 
                    e.duration.as_millis()
                );
            }
            AgentEvent::AgentCompleted(e) => {
                println!("🎉 Agent completed in {}ms", e.duration.as_millis());
            }
            _ => {}
        }
        Ok(())
    }
    
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Only log major lifecycle events
        matches!(
            event,
            AgentEvent::AgentStarted(_)
                | AgentEvent::AgentCompleted(_)
                | AgentEvent::ToolStarted(_)
                | AgentEvent::ToolCompleted(_)
        )
    }
}
```

### 2. WhatsApp Broadcaster

Send progress updates to customers via WhatsApp:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;

pub struct WhatsAppBroadcaster {
    customer_phone: String,
    whatsapp_client: WhatsAppClient,
}

impl WhatsAppBroadcaster {
    pub fn new(customer_phone: String, whatsapp_client: WhatsAppClient) -> Self {
        Self {
            customer_phone,
            whatsapp_client,
        }
    }
    
    fn format_message(&self, event: &AgentEvent) -> Option<String> {
        match event {
            AgentEvent::SubAgentStarted(e) => {
                match e.agent_name.as_str() {
                    "diagnostic-agent" => Some("🔍 Analyzing your vehicle issue...".to_string()),
                    "quote-agent" => Some("💰 Getting quotes from garages...".to_string()),
                    _ => None,
                }
            }
            AgentEvent::TodosUpdated(e) => {
                let completed = e.completed_count;
                let total = e.todos.len();
                Some(format!("✅ Progress: {}/{} steps completed", completed, total))
            }
            _ => None,
        }
    }
}

#[async_trait]
impl EventBroadcaster for WhatsAppBroadcaster {
    fn id(&self) -> &str {
        "whatsapp"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        if let Some(message) = self.format_message(event) {
            self.whatsapp_client
                .send_text(&self.customer_phone, &message)
                .await?;
            
            tracing::info!(
                customer_phone = %self.customer_phone,
                "Sent WhatsApp progress update"
            );
        }
        Ok(())
    }
    
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Only send user-facing events to WhatsApp
        matches!(
            event,
            AgentEvent::SubAgentStarted(_) | AgentEvent::TodosUpdated(_)
        )
    }
}
```

### 3. SSE (Server-Sent Events) Broadcaster

Stream events to web clients:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;
use tokio::sync::broadcast;

pub struct SseBroadcaster {
    sender: broadcast::Sender<String>,
    thread_id: String,
}

impl SseBroadcaster {
    pub fn new(sender: broadcast::Sender<String>, thread_id: String) -> Self {
        Self { sender, thread_id }
    }
}

#[async_trait]
impl EventBroadcaster for SseBroadcaster {
    fn id(&self) -> &str {
        "sse"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Serialize event to JSON
        let json = serde_json::to_string(event)?;
        
        // Send to SSE channel (non-blocking)
        let _ = self.sender.send(json);
        
        tracing::debug!(
            thread_id = %self.thread_id,
            "Sent SSE event"
        );
        
        Ok(())
    }
}
```

### 4. DynamoDB Broadcaster

Persist events to DynamoDB for audit trail:

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;
use aws_sdk_dynamodb::Client as DynamoDbClient;

pub struct DynamoDbBroadcaster {
    client: DynamoDbClient,
    table_name: String,
    customer_id: String,
}

impl DynamoDbBroadcaster {
    pub fn new(
        client: DynamoDbClient,
        table_name: String,
        customer_id: String,
    ) -> Self {
        Self {
            client,
            table_name,
            customer_id,
        }
    }
}

#[async_trait]
impl EventBroadcaster for DynamoDbBroadcaster {
    fn id(&self) -> &str {
        "dynamodb"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        let event_json = serde_json::to_string(event)?;
        let timestamp = chrono::Utc::now().to_rfc3339();
        let event_id = uuid::Uuid::new_v4().to_string();
        
        self.client
            .put_item()
            .table_name(&self.table_name)
            .item(
                "PK",
                aws_sdk_dynamodb::types::AttributeValue::S(
                    format!("CUSTOMER#{}", self.customer_id)
                ),
            )
            .item(
                "SK",
                aws_sdk_dynamodb::types::AttributeValue::S(
                    format!("EVENT#{}", timestamp)
                ),
            )
            .item(
                "event_id",
                aws_sdk_dynamodb::types::AttributeValue::S(event_id),
            )
            .item(
                "event_data",
                aws_sdk_dynamodb::types::AttributeValue::S(event_json),
            )
            .send()
            .await?;
        
        Ok(())
    }
}
```

## Using the Event System

### Basic Setup

```rust
use agents_runtime::agent::DeepAgentBuilder;
use agents_core::events::EventDispatcher;
use std::sync::Arc;

// Create event dispatcher
let mut dispatcher = EventDispatcher::new();

// Add broadcasters
dispatcher.add_broadcaster(Arc::new(ConsoleLoggerBroadcaster));
dispatcher.add_broadcaster(Arc::new(WhatsAppBroadcaster::new(
    customer_phone,
    whatsapp_client,
)));

// Build agent with event dispatcher
let agent = DeepAgentBuilder::new("my-agent")
    .with_event_dispatcher(Arc::new(dispatcher))
    .build()?;

// Events are now automatically emitted during agent execution
let response = agent.handle_message(input).await?;
```

### Adding Individual Broadcasters

```rust
let agent = DeepAgentBuilder::new("my-agent")
    .with_event_broadcaster(Arc::new(ConsoleLoggerBroadcaster))
    .with_event_broadcaster(Arc::new(SseBroadcaster::new(sse_sender, thread_id)))
    .build()?;
```

### Multi-Channel Broadcasting

```rust
// Create dispatcher with multiple channels
let mut dispatcher = EventDispatcher::new();

// Add WhatsApp for customer updates
dispatcher.add_broadcaster(Arc::new(WhatsAppBroadcaster::new(
    customer_phone,
    whatsapp_client,
)));

// Add SSE for web portal
dispatcher.add_broadcaster(Arc::new(SseBroadcaster::new(
    sse_sender,
    thread_id.clone(),
)));

// Add DynamoDB for audit trail
dispatcher.add_broadcaster(Arc::new(DynamoDbBroadcaster::new(
    dynamodb_client,
    "agent-events".to_string(),
    customer_id,
)));

// All channels receive events simultaneously
let agent = DeepAgentBuilder::new("my-agent")
    .with_event_dispatcher(Arc::new(dispatcher))
    .build()?;
```

## Performance Considerations

### Non-Blocking Design

Events are dispatched asynchronously using `tokio::spawn`:

```rust
pub async fn dispatch(&self, event: AgentEvent) {
    let broadcasters = self.broadcasters.clone();
    
    // Spawn tasks for each broadcaster to avoid blocking
    for broadcaster in broadcasters {
        let event_clone = event.clone();
        tokio::spawn(async move {
            if broadcaster.should_broadcast(&event_clone) {
                if let Err(e) = broadcaster.broadcast(&event_clone).await {
                    tracing::warn!(
                        broadcaster_id = broadcaster.id(),
                        error = %e,
                        "Failed to broadcast event"
                    );
                }
            }
        });
    }
}
```

### Overhead Metrics

- Event creation: ~1µs (struct allocation)
- Serialization: ~10µs (serde_json)
- Dispatch spawn: ~5µs (tokio::spawn)
- **Total overhead: <20µs per event**

### Memory Usage

- Event struct: ~500 bytes
- Dispatcher: ~1KB + (N broadcasters × 100 bytes)
- **Total per agent: <2KB**

## Error Handling

### Broadcaster Failures

Broadcaster failures are isolated and logged but never affect agent execution:

```rust
if let Err(e) = broadcaster.broadcast(&event).await {
    tracing::warn!(
        broadcaster_id = broadcaster.id(),
        error = %e,
        "Failed to broadcast event"
    );
}
// Agent continues execution
```

### Best Practices

1. **Graceful Degradation**: Handle network failures in broadcasters
2. **Timeouts**: Add timeouts to external API calls
3. **Rate Limiting**: Implement rate limiting for high-frequency events
4. **Retry Logic**: Add retries for transient failures (optional)

## Testing

### Mock Broadcaster

```rust
use std::sync::{Arc, Mutex};

pub struct MockBroadcaster {
    events: Arc<Mutex<Vec<AgentEvent>>>,
}

impl MockBroadcaster {
    pub fn new() -> Self {
        Self {
            events: Arc::new(Mutex::new(Vec::new())),
        }
    }
    
    pub fn get_events(&self) -> Vec<AgentEvent> {
        self.events.lock().unwrap().clone()
    }
}

#[async_trait]
impl EventBroadcaster for MockBroadcaster {
    fn id(&self) -> &str {
        "mock"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        self.events.lock().unwrap().push(event.clone());
        Ok(())
    }
}

// In tests
#[tokio::test]
async fn test_events_emitted() {
    let mock = Arc::new(MockBroadcaster::new());
    
    let agent = DeepAgentBuilder::new("test-agent")
        .with_event_broadcaster(mock.clone())
        .build()?;
    
    agent.handle_message(input).await?;
    
    let events = mock.get_events();
    assert!(events.iter().any(|e| matches!(e, AgentEvent::AgentStarted(_))));
    assert!(events.iter().any(|e| matches!(e, AgentEvent::AgentCompleted(_))));
}
```

## Security Considerations

### PII Protection

Always truncate sensitive data in events:

```rust
fn truncate_message(msg: &str) -> String {
    if msg.len() > 100 {
        format!("{}...", &msg[..100])
    } else {
        msg.to_string()
    }
}
```

### Access Control

- Verify thread_id ownership before SSE subscriptions
- Scope DynamoDB items to customer_id
- Validate phone numbers before WhatsApp broadcasts

## Monitoring

### Metrics

Expose these metrics for observability:

- `agent_events_emitted_total{event_type}`: Counter
- `broadcaster_failures_total{broadcaster_id, event_type}`: Counter
- `broadcast_latency_seconds{broadcaster_id}`: Histogram

### Structured Logging

Use structured logging with correlation IDs:

```rust
tracing::info!(
    thread_id = %metadata.thread_id,
    correlation_id = %metadata.correlation_id,
    event_type = "tool_started",
    tool_name = %tool_name,
    "Tool execution started"
);
```

## See Also

- [Migration Guide](./MIGRATION_GUIDE.md) - Migrating from agent_progress_subscriber
- [Event System Demo](../examples/event-system-demo/) - Complete working example
- [API Reference](https://docs.rs/agents-core/latest/agents_core/events/) - Full API documentation
