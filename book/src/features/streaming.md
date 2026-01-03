# Streaming

Real-time response streaming for interactive applications.

## Overview

Streaming enables:
- **Immediate feedback** as responses generate
- **Better UX** for chat interfaces
- **Progress indication** for long operations
- **Reduced perceived latency**

## Quick Start

```rust
use agents_sdk::events::{EventDispatcher, AgentEvent};

let dispatcher = Arc::new(EventDispatcher::new());
let mut receiver = dispatcher.subscribe();

let agent = ConfigurableAgentBuilder::new("...")
    .with_model(model)
    .with_event_dispatcher(dispatcher)
    .build()?;

// Listen for streaming tokens
tokio::spawn(async move {
    while let Ok(event) = receiver.recv().await {
        if let AgentEvent::StreamingToken(token) = event {
            print!("{}", token.content);
            std::io::stdout().flush().unwrap();
        }
    }
});

// Send message (streams automatically)
let response = agent.handle_message("Tell me a story", state).await?;
```

## StreamingTokenEvent

```rust
pub struct StreamingTokenEvent {
    pub metadata: EventMetadata,
    pub content: String,   // Token chunk
    pub index: u32,        // Position in stream
}
```

## Web Streaming

### Server-Sent Events (SSE)

```rust
use axum::{
    response::sse::{Event, Sse},
    extract::State,
};
use futures::stream::Stream;

async fn stream_response(
    State(app_state): State<AppState>,
    Json(request): Json<ChatRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let dispatcher = Arc::new(EventDispatcher::new());
    let mut receiver = dispatcher.subscribe();
    
    let agent = app_state.create_agent(dispatcher.clone());
    
    // Start processing in background
    let state = Arc::new(AgentStateSnapshot::default());
    tokio::spawn(async move {
        let _ = agent.handle_message(&request.message, state).await;
    });
    
    // Stream events
    let stream = async_stream::stream! {
        while let Ok(event) = receiver.recv().await {
            match event {
                AgentEvent::StreamingToken(t) => {
                    yield Ok(Event::default()
                        .event("token")
                        .data(t.content));
                }
                AgentEvent::ToolStarted(t) => {
                    yield Ok(Event::default()
                        .event("tool_start")
                        .data(t.tool_name));
                }
                AgentEvent::ToolCompleted(t) => {
                    yield Ok(Event::default()
                        .event("tool_complete")
                        .data(serde_json::to_string(&t).unwrap()));
                }
                AgentEvent::AgentCompleted(_) => {
                    yield Ok(Event::default()
                        .event("done")
                        .data(""));
                    break;
                }
                _ => {}
            }
        }
    };
    
    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::default()
    )
}
```

### Frontend (JavaScript)

```javascript
const eventSource = new EventSource('/api/chat/stream');

eventSource.addEventListener('token', (e) => {
    // Append token to output
    output.textContent += e.data;
});

eventSource.addEventListener('tool_start', (e) => {
    // Show tool indicator
    showToolIndicator(e.data);
});

eventSource.addEventListener('tool_complete', (e) => {
    // Hide tool indicator
    hideToolIndicator();
});

eventSource.addEventListener('done', () => {
    eventSource.close();
});
```

### WebSocket

```rust
use axum::extract::ws::{WebSocket, WebSocketUpgrade, Message};

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(app_state): State<AppState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| handle_ws(socket, app_state))
}

async fn handle_ws(mut socket: WebSocket, app_state: AppState) {
    while let Some(Ok(msg)) = socket.recv().await {
        if let Message::Text(text) = msg {
            let request: ChatRequest = serde_json::from_str(&text).unwrap();
            
            let dispatcher = Arc::new(EventDispatcher::new());
            let mut receiver = dispatcher.subscribe();
            
            let agent = app_state.create_agent(dispatcher.clone());
            let state = Arc::new(AgentStateSnapshot::default());
            
            // Process in background
            tokio::spawn(async move {
                let _ = agent.handle_message(&request.message, state).await;
            });
            
            // Stream events via WebSocket
            while let Ok(event) = receiver.recv().await {
                let json = serde_json::to_string(&event).unwrap();
                if socket.send(Message::Text(json)).await.is_err() {
                    break;
                }
                
                if matches!(event, AgentEvent::AgentCompleted(_)) {
                    break;
                }
            }
        }
    }
}
```

## CLI Streaming

```rust
use std::io::{self, Write};

async fn stream_to_cli(mut receiver: broadcast::Receiver<AgentEvent>) {
    let mut in_tool = false;
    
    while let Ok(event) = receiver.recv().await {
        match event {
            AgentEvent::StreamingToken(t) => {
                if !in_tool {
                    print!("{}", t.content);
                    io::stdout().flush().unwrap();
                }
            }
            AgentEvent::ToolStarted(t) => {
                in_tool = true;
                println!("\n🔧 Using {}...", t.tool_name);
            }
            AgentEvent::ToolCompleted(t) => {
                in_tool = false;
                println!("✅ {} complete ({} ms)\n", t.tool_name, t.duration_ms);
            }
            AgentEvent::AgentCompleted(_) => {
                println!();
                break;
            }
            _ => {}
        }
    }
}
```

## Progress Indicators

Show meaningful progress during tool execution:

```rust
async fn stream_with_progress(mut receiver: broadcast::Receiver<AgentEvent>) {
    let spinner = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
    let mut spinner_idx = 0;
    
    while let Ok(event) = receiver.recv().await {
        match event {
            AgentEvent::StreamingToken(t) => {
                print!("{}", t.content);
                io::stdout().flush().unwrap();
            }
            AgentEvent::ToolStarted(t) => {
                // Start spinner
                print!("\r{} {} ", spinner[spinner_idx], t.tool_name);
                spinner_idx = (spinner_idx + 1) % spinner.len();
            }
            AgentEvent::ToolCompleted(t) => {
                // Clear spinner, show completion
                print!("\r✓ {} ({} ms)          \n", t.tool_name, t.duration_ms);
            }
            AgentEvent::AgentCompleted(_) => break,
            _ => {}
        }
    }
}
```

## Buffered Streaming

Buffer tokens for smoother display:

```rust
use tokio::time::{interval, Duration};

async fn buffered_stream(mut receiver: broadcast::Receiver<AgentEvent>) {
    let mut buffer = String::new();
    let mut flush_interval = interval(Duration::from_millis(50));
    
    loop {
        tokio::select! {
            Ok(event) = receiver.recv() => {
                match event {
                    AgentEvent::StreamingToken(t) => {
                        buffer.push_str(&t.content);
                    }
                    AgentEvent::AgentCompleted(_) => {
                        // Flush remaining
                        if !buffer.is_empty() {
                            print!("{}", buffer);
                            io::stdout().flush().unwrap();
                        }
                        break;
                    }
                    _ => {}
                }
            }
            _ = flush_interval.tick() => {
                if !buffer.is_empty() {
                    print!("{}", buffer);
                    io::stdout().flush().unwrap();
                    buffer.clear();
                }
            }
        }
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
use std::io::{self, Write};

#[tool("Search for information")]
async fn search(query: String) -> String {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    format!("Found results for: {}", query)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);

    let dispatcher = Arc::new(EventDispatcher::new());
    let mut receiver = dispatcher.subscribe();

    // Spawn stream handler
    let stream_handle = tokio::spawn(async move {
        let mut tool_active = false;
        
        while let Ok(event) = receiver.recv().await {
            match event {
                AgentEvent::StreamingToken(t) => {
                    if !tool_active {
                        print!("{}", t.content);
                        io::stdout().flush().unwrap();
                    }
                }
                AgentEvent::ToolStarted(t) => {
                    tool_active = true;
                    println!("\n\n🔧 Executing: {}", t.tool_name);
                }
                AgentEvent::ToolCompleted(t) => {
                    tool_active = false;
                    println!("✅ Complete: {} ({}ms)\n", t.tool_name, t.duration_ms);
                }
                AgentEvent::AgentCompleted(c) => {
                    println!("\n\n---\nCompleted in {}ms", c.duration_ms);
                    break;
                }
                _ => {}
            }
        }
    });

    let agent = ConfigurableAgentBuilder::new("You are a helpful assistant.")
        .with_model(model)
        .with_tool(SearchTool::as_tool())
        .with_event_dispatcher(dispatcher)
        .build()?;

    println!("🤖 Agent: ");
    
    agent.handle_message(
        "Search for Rust programming and tell me about it",
        Arc::new(AgentStateSnapshot::default())
    ).await?;

    stream_handle.await?;

    Ok(())
}
```

## Best Practices

### 1. Handle Disconnects

```rust
if socket.send(message).await.is_err() {
    // Client disconnected
    tracing::info!("Client disconnected during stream");
    break;
}
```

### 2. Set Timeouts

```rust
use tokio::time::timeout;

let result = timeout(
    Duration::from_secs(60),
    receiver.recv()
).await;
```

### 3. Indicate Tool Usage

```rust
// Don't stream during tool execution
// Instead show a status indicator
```

### 4. Buffer for Smoothness

```rust
// Buffer ~50ms worth of tokens for smooth display
```

