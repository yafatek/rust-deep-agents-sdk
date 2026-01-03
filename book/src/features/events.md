# Event System

Real-time event broadcasting for monitoring, logging, and streaming.

## Overview

The event system provides:
- **Real-time updates**: Stream progress to clients
- **Observability**: Monitor agent behavior
- **Analytics**: Track usage patterns
- **Debugging**: Trace execution flow

## Event Types

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

## Quick Start

```rust
use agents_sdk::events::{EventDispatcher, AgentEvent};
use std::sync::Arc;

let dispatcher = Arc::new(EventDispatcher::new());
let mut receiver = dispatcher.subscribe();

let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_event_dispatcher(dispatcher)
    .build()?;

// Listen to events
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        match event {
            AgentEvent::AgentStarted(e) => {
                println!("Agent started: {}", e.agent_name);
            }
            AgentEvent::ToolCompleted(e) => {
                println!("Tool {} completed in {}ms", e.tool_name, e.duration_ms);
            }
            AgentEvent::TokenUsage(e) => {
                println!("Tokens: {}", e.usage.total_tokens);
            }
            _ => {}
        }
    }
});
```

## Event Details

### AgentStartedEvent

```rust
pub struct AgentStartedEvent {
    pub metadata: EventMetadata,
    pub agent_name: String,
    pub message_preview: String,
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

### ToolStartedEvent / ToolCompletedEvent

```rust
pub struct ToolStartedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub args_preview: String,
}

pub struct ToolCompletedEvent {
    pub metadata: EventMetadata,
    pub tool_name: String,
    pub duration_ms: u64,
    pub result_preview: String,
}
```

### StreamingTokenEvent

```rust
pub struct StreamingTokenEvent {
    pub metadata: EventMetadata,
    pub content: String,  // Token chunk
    pub index: u32,       // Position in stream
}
```

### TokenUsageEvent

```rust
pub struct TokenUsageEvent {
    pub metadata: EventMetadata,
    pub usage: TokenUsage,
}
```

## Event Metadata

All events include metadata:

```rust
pub struct EventMetadata {
    pub thread_id: String,
    pub correlation_id: String,
    pub customer_id: Option<String>,
    pub timestamp: String,
}
```

## Streaming to Web Clients

### Server-Sent Events (SSE)

```rust
use axum::{
    response::sse::{Event, Sse},
    extract::State,
};
use futures::stream::Stream;
use std::convert::Infallible;

async fn sse_handler(
    State(dispatcher): State<Arc<EventDispatcher>>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let mut receiver = dispatcher.subscribe();
    
    let stream = async_stream::stream! {
        while let Ok(event) = receiver.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            yield Ok(Event::default().data(json));
        }
    };
    
    Sse::new(stream)
}
```

### WebSocket

```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(dispatcher): State<Arc<EventDispatcher>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, dispatcher))
}

async fn handle_socket(mut socket: WebSocket, dispatcher: Arc<EventDispatcher>) {
    let mut receiver = dispatcher.subscribe();
    
    while let Ok(event) = receiver.recv().await {
        let json = serde_json::to_string(&event).unwrap();
        if socket.send(Message::Text(json)).await.is_err() {
            break;
        }
    }
}
```

## Custom Event Broadcasting

Implement the `EventBroadcaster` trait:

```rust
use agents_core::events::{EventBroadcaster, AgentEvent};
use async_trait::async_trait;

struct WebhookBroadcaster {
    endpoint: String,
    client: reqwest::Client,
}

#[async_trait]
impl EventBroadcaster for WebhookBroadcaster {
    fn id(&self) -> &str {
        "webhook"
    }
    
    fn supports_streaming(&self) -> bool {
        false
    }
    
    async fn broadcast(&self, event: &AgentEvent) -> anyhow::Result<()> {
        self.client
            .post(&self.endpoint)
            .json(event)
            .send()
            .await?;
        Ok(())
    }
}
```

Use with the builder:

```rust
let webhook = Arc::new(WebhookBroadcaster {
    endpoint: "https://api.example.com/events".to_string(),
    client: reqwest::Client::new(),
});

let agent = ConfigurableAgentBuilder::new("...")
    .with_event_broadcaster(webhook)
    .build()?;
```

## Filtering Events

Process only relevant events:

```rust
while let Ok(event) = receiver.recv().await {
    // Filter by type
    match &event {
        AgentEvent::ToolCompleted(_) | AgentEvent::TokenUsage(_) => {
            process_event(&event).await;
        }
        _ => {} // Ignore others
    }
    
    // Filter by metadata
    if event.metadata().thread_id == "important-thread" {
        special_handling(&event).await;
    }
}
```

## Complete Example

```rust
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    events::{EventDispatcher, AgentEvent},
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;
use tokio::sync::mpsc;

#[tool("Search for information")]
async fn search(query: String) -> String {
    tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    format!("Results for: {}", query)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let dispatcher = Arc::new(EventDispatcher::new());
    let mut receiver = dispatcher.subscribe();

    // Event processor
    let event_handle = tokio::spawn(async move {
        let mut tool_count = 0;
        let mut total_tokens = 0;
        
        while let Ok(event) = receiver.recv().await {
            match event {
                AgentEvent::AgentStarted(e) => {
                    println!("🚀 Started: {}", e.message_preview);
                }
                AgentEvent::ToolStarted(e) => {
                    println!("🔧 Tool starting: {}", e.tool_name);
                }
                AgentEvent::ToolCompleted(e) => {
                    tool_count += 1;
                    println!("✅ Tool completed: {} ({}ms)", e.tool_name, e.duration_ms);
                }
                AgentEvent::StreamingToken(e) => {
                    print!("{}", e.content);
                }
                AgentEvent::TokenUsage(e) => {
                    total_tokens += e.usage.total_tokens;
                    println!("\n📊 Tokens: {}", e.usage.total_tokens);
                }
                AgentEvent::AgentCompleted(e) => {
                    println!("🏁 Completed in {}ms", e.duration_ms);
                    println!("   Tools called: {}", tool_count);
                    println!("   Total tokens: {}", total_tokens);
                }
                _ => {}
            }
        }
    });

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_tool(SearchTool::as_tool())
        .with_event_dispatcher(dispatcher)
        .with_token_tracking(true)
        .build()?;

    let _response = agent.handle_message(
        "Search for information about Rust programming language",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    // Wait for event processing
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;

    Ok(())
}
```

## Best Practices

### 1. Handle Receiver Drops

```rust
while let Ok(event) = receiver.recv().await {
    if let Err(e) = process_event(&event).await {
        tracing::error!("Event processing error: {}", e);
        // Continue processing other events
    }
}
```

### 2. Use Correlation IDs

```rust
// Track related events
let correlation_id = event.metadata().correlation_id.clone();
tracing::info!(correlation_id = %correlation_id, "Processing event");
```

### 3. Buffer for High Throughput

```rust
let (tx, mut rx) = mpsc::channel(1000);  // Buffer size

tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        if tx.send(event).await.is_err() {
            break;
        }
    }
});

// Process buffered events
while let Some(event) = rx.recv().await {
    batch_process(event).await;
}
```

### 4. Selective Streaming

```rust
// Only stream user-visible events
fn should_stream(event: &AgentEvent) -> bool {
    matches!(event, 
        AgentEvent::StreamingToken(_) |
        AgentEvent::ToolStarted(_) |
        AgentEvent::ToolCompleted(_)
    )
}
```

