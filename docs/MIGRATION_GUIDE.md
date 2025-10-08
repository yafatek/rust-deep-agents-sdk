# Migration Guide: agent_progress_subscriber to Event System

## Overview

This guide helps you migrate from the deprecated `agent_progress_subscriber` module to the new event system. The new system provides better flexibility, multi-channel broadcasting, and improved performance.

## Why Migrate?

### Old System Limitations

- **Single Channel**: Only supported one progress subscriber at a time
- **Tightly Coupled**: Progress logic mixed with agent runtime
- **Limited Events**: Only basic progress updates
- **No Filtering**: All events sent to subscriber

### New System Benefits

- **Multi-Channel**: Broadcast to WhatsApp, SSE, DynamoDB simultaneously
- **Extensible**: Easy to add custom broadcasters
- **Rich Events**: 10+ event types covering full agent lifecycle
- **Selective Broadcasting**: Filter events per broadcaster
- **Non-Blocking**: Zero impact on agent performance
- **Type-Safe**: Strongly typed event structures

## Migration Steps

### Step 1: Update Dependencies

The event system is included in `agents-core` and `agents-runtime`, so no new dependencies are needed if you're already using the SDK.

```toml
[dependencies]
agents-core = "0.0.16"
agents-runtime = "0.0.16"
```

### Step 2: Replace Progress Subscriber

#### Old Code (Deprecated)

```rust
use agents_runtime::agent_progress_subscriber::{
    AgentProgressSubscriber,
    ProgressUpdate,
};

struct MyProgressSubscriber;

impl AgentProgressSubscriber for MyProgressSubscriber {
    fn on_progress(&self, update: ProgressUpdate) {
        println!("Progress: {:?}", update);
    }
}

let agent = DeepAgentBuilder::new("my-agent")
    .with_progress_subscriber(Box::new(MyProgressSubscriber))
    .build()?;
```

#### New Code (Event System)

```rust
use agents_core::events::{AgentEvent, EventBroadcaster};
use async_trait::async_trait;

struct MyEventBroadcaster;

#[async_trait]
impl EventBroadcaster for MyEventBroadcaster {
    fn id(&self) -> &str {
        "my_broadcaster"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        println!("Event: {:?}", event);
        Ok(())
    }
}

let agent = DeepAgentBuilder::new("my-agent")
    .with_event_broadcaster(Arc::new(MyEventBroadcaster))
    .build()?;
```

### Step 3: Map Progress Updates to Events

#### Progress Update Types → Event Types

| Old ProgressUpdate | New AgentEvent | Notes |
|-------------------|----------------|-------|
| `AgentStarted` | `AgentEvent::AgentStarted` | Same concept |
| `ToolExecuting` | `AgentEvent::ToolStarted` | Renamed for clarity |
| `ToolCompleted` | `AgentEvent::ToolCompleted` | Now includes duration |
| `AgentCompleted` | `AgentEvent::AgentCompleted` | Now includes duration |
| N/A | `AgentEvent::ToolFailed` | New: explicit error events |
| N/A | `AgentEvent::SubAgentStarted` | New: sub-agent tracking |
| N/A | `AgentEvent::SubAgentCompleted` | New: sub-agent tracking |
| N/A | `AgentEvent::TodosUpdated` | New: todo list changes |
| N/A | `AgentEvent::StateCheckpointed` | New: persistence events |
| N/A | `AgentEvent::PlanningComplete` | New: planner decisions |

### Step 4: Handle Async Broadcasting

The old system used synchronous callbacks. The new system is fully async:

#### Old Code

```rust
impl AgentProgressSubscriber for MySubscriber {
    fn on_progress(&self, update: ProgressUpdate) {
        // Synchronous - blocks agent execution
        send_notification(&update);
    }
}
```

#### New Code

```rust
#[async_trait]
impl EventBroadcaster for MyBroadcaster {
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Async - non-blocking
        send_notification(event).await?;
        Ok(())
    }
}
```

### Step 5: Implement Event Filtering

The new system allows selective event broadcasting:

```rust
#[async_trait]
impl EventBroadcaster for MyBroadcaster {
    fn id(&self) -> &str {
        "my_broadcaster"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Handle event
        Ok(())
    }
    
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Only broadcast user-facing events
        matches!(
            event,
            AgentEvent::AgentStarted(_)
                | AgentEvent::ToolStarted(_)
                | AgentEvent::AgentCompleted(_)
        )
    }
}
```

## Common Migration Patterns

### Pattern 1: Console Logging

#### Before

```rust
struct ConsoleLogger;

impl AgentProgressSubscriber for ConsoleLogger {
    fn on_progress(&self, update: ProgressUpdate) {
        match update {
            ProgressUpdate::AgentStarted { agent_name } => {
                println!("Agent started: {}", agent_name);
            }
            ProgressUpdate::ToolExecuting { tool_name } => {
                println!("Tool executing: {}", tool_name);
            }
            _ => {}
        }
    }
}
```

#### After

```rust
struct ConsoleLogger;

#[async_trait]
impl EventBroadcaster for ConsoleLogger {
    fn id(&self) -> &str {
        "console"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        match event {
            AgentEvent::AgentStarted(e) => {
                println!("Agent started: {}", e.agent_name);
            }
            AgentEvent::ToolStarted(e) => {
                println!("Tool started: {}", e.tool_name);
            }
            _ => {}
        }
        Ok(())
    }
}
```

### Pattern 2: External API Notifications

#### Before

```rust
struct ApiNotifier {
    client: reqwest::Client,
    webhook_url: String,
}

impl AgentProgressSubscriber for ApiNotifier {
    fn on_progress(&self, update: ProgressUpdate) {
        // Blocking HTTP call - bad!
        let _ = self.client
            .post(&self.webhook_url)
            .json(&update)
            .send();
    }
}
```

#### After

```rust
struct ApiNotifier {
    client: reqwest::Client,
    webhook_url: String,
}

#[async_trait]
impl EventBroadcaster for ApiNotifier {
    fn id(&self) -> &str {
        "api_notifier"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        // Non-blocking async HTTP call
        self.client
            .post(&self.webhook_url)
            .json(event)
            .send()
            .await?;
        Ok(())
    }
}
```

### Pattern 3: Multi-Channel Broadcasting

The old system only supported one subscriber. The new system supports multiple:

#### Before (Not Possible)

```rust
// Could only have ONE subscriber
let agent = DeepAgentBuilder::new("my-agent")
    .with_progress_subscriber(Box::new(ConsoleLogger))
    // Can't add another subscriber!
    .build()?;
```

#### After (Multiple Broadcasters)

```rust
let agent = DeepAgentBuilder::new("my-agent")
    .with_event_broadcaster(Arc::new(ConsoleLogger))
    .with_event_broadcaster(Arc::new(ApiNotifier::new(client, url)))
    .with_event_broadcaster(Arc::new(WhatsAppBroadcaster::new(phone)))
    .build()?;
```

## Advanced Migration Scenarios

### Scenario 1: State-Based Progress Tracking

If you were tracking state in your progress subscriber:

#### Before

```rust
struct StatefulSubscriber {
    state: Arc<Mutex<HashMap<String, ProgressState>>>,
}

impl AgentProgressSubscriber for StatefulSubscriber {
    fn on_progress(&self, update: ProgressUpdate) {
        let mut state = self.state.lock().unwrap();
        // Update state based on progress
    }
}
```

#### After

```rust
struct StatefulBroadcaster {
    state: Arc<Mutex<HashMap<String, EventState>>>,
}

#[async_trait]
impl EventBroadcaster for StatefulBroadcaster {
    fn id(&self) -> &str {
        "stateful"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        let mut state = self.state.lock().unwrap();
        // Update state based on event
        // Now with richer event data!
        Ok(())
    }
}
```

### Scenario 2: Conditional Notifications

#### Before

```rust
impl AgentProgressSubscriber for ConditionalNotifier {
    fn on_progress(&self, update: ProgressUpdate) {
        if self.should_notify(&update) {
            self.send_notification(&update);
        }
    }
}
```

#### After (Use should_broadcast)

```rust
#[async_trait]
impl EventBroadcaster for ConditionalNotifier {
    fn id(&self) -> &str {
        "conditional"
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        self.send_notification(event).await?;
        Ok(())
    }
    
    fn should_broadcast(&self, event: &AgentEvent) -> bool {
        // Filter logic here - more efficient!
        self.should_notify(event)
    }
}
```

## Testing Migration

### Test Old and New Side-by-Side

During migration, you can run both systems temporarily:

```rust
// Keep old system temporarily
let agent = DeepAgentBuilder::new("my-agent")
    .with_progress_subscriber(Box::new(OldSubscriber))
    .with_event_broadcaster(Arc::new(NewBroadcaster))
    .build()?;

// Verify both receive same information
// Then remove old subscriber
```

### Unit Test Your Broadcaster

```rust
#[tokio::test]
async fn test_my_broadcaster() {
    let broadcaster = MyBroadcaster::new();
    
    let event = AgentEvent::AgentStarted(AgentStartedEvent {
        metadata: EventMetadata {
            thread_id: "test".to_string(),
            correlation_id: "test".to_string(),
            customer_id: None,
            timestamp: chrono::Utc::now(),
        },
        agent_name: "test-agent".to_string(),
        message_preview: "test message".to_string(),
    });
    
    // Should not panic
    broadcaster.broadcast(&event).await.unwrap();
}
```

## Troubleshooting

### Issue: Events Not Received

**Symptom**: Broadcaster not receiving events

**Solution**: Ensure broadcaster is added before agent execution:

```rust
let agent = DeepAgentBuilder::new("my-agent")
    .with_event_broadcaster(Arc::new(MyBroadcaster))  // Add BEFORE build()
    .build()?;
```

### Issue: Blocking Agent Execution

**Symptom**: Agent becomes slow after adding broadcaster

**Solution**: Ensure broadcaster is truly async and doesn't block:

```rust
async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
    // BAD: Blocking operation
    // std::thread::sleep(Duration::from_secs(1));
    
    // GOOD: Async operation
    tokio::time::sleep(Duration::from_secs(1)).await;
    Ok(())
}
```

### Issue: Missing Events

**Symptom**: Some events not received

**Solution**: Check `should_broadcast()` filter:

```rust
fn should_broadcast(&self, event: &AgentEvent) -> bool {
    // Make sure you're not filtering out events you need!
    true  // Broadcast all events during debugging
}
```

## Deprecation Timeline

- **v0.0.16**: Event system introduced, `agent_progress_subscriber` marked deprecated
- **v0.0.18**: Deprecation warnings added
- **v0.0.18**: `agent_progress_subscriber` removed

## Getting Help

- See [Event System Documentation](./EVENT_SYSTEM.md) for full API reference
- Check [examples/event-system-demo](../examples/event-system-demo/) for working code
- Open an issue on GitHub for migration questions

## Checklist

Use this checklist to track your migration:

- [ ] Updated to agents-core 0.0.16+
- [ ] Replaced `AgentProgressSubscriber` with `EventBroadcaster`
- [ ] Made broadcast method async
- [ ] Implemented `should_broadcast()` for filtering
- [ ] Added error handling in broadcast method
- [ ] Tested with mock events
- [ ] Verified no blocking operations
- [ ] Removed old progress subscriber code
- [ ] Updated documentation
- [ ] Deployed and monitored

## Summary

The new event system provides:

✅ **Better Performance**: Non-blocking async design  
✅ **More Flexibility**: Multiple broadcasters, selective filtering  
✅ **Richer Data**: 10+ event types with detailed metadata  
✅ **Easier Testing**: Mock broadcasters for unit tests  
✅ **Production Ready**: Used in production at scale  

Migration is straightforward and can be done incrementally. Start with a simple broadcaster, test thoroughly, then expand to multiple channels.
