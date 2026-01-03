# Web Service Example

Full-stack web application with Axum backend and React frontend.

## Overview

The `automotive-web-service` example demonstrates:
- REST API with Axum
- Server-Sent Events streaming
- React frontend with real-time updates
- Production patterns

## Architecture

```
┌─────────────────────────────────────────────────┐
│                   Frontend                      │
│              (React + Vite)                     │
├─────────────────────────────────────────────────┤
│  - Chat interface                               │
│  - SSE event subscription                       │
│  - Real-time streaming display                  │
└───────────────────────┬─────────────────────────┘
                        │ HTTP/SSE
┌───────────────────────▼─────────────────────────┐
│                   Backend                       │
│               (Axum + Tokio)                    │
├─────────────────────────────────────────────────┤
│  POST /api/chat      - Send message             │
│  GET  /api/events    - SSE stream               │
│  GET  /api/health    - Health check             │
└───────────────────────┬─────────────────────────┘
                        │
┌───────────────────────▼─────────────────────────┐
│              Deep Agents SDK                    │
├─────────────────────────────────────────────────┤
│  - ConfigurableAgentBuilder                     │
│  - EventDispatcher                              │
│  - Tool execution                               │
└─────────────────────────────────────────────────┘
```

## Backend Code

```rust
use axum::{
    extract::State,
    response::sse::{Event, Sse},
    routing::{get, post},
    Json, Router,
};
use agents_sdk::{
    ConfigurableAgentBuilder,
    OpenAiConfig,
    OpenAiChatModel,
    events::{EventDispatcher, AgentEvent},
    tool,
    state::AgentStateSnapshot,
};
use std::sync::Arc;
use tokio::sync::broadcast;

struct AppState {
    agent: Arc<DeepAgent>,
    event_tx: broadcast::Sender<AgentEvent>,
}

#[tool("Search for vehicles")]
async fn search_vehicles(make: String, model: Option<String>) -> String {
    // Vehicle search implementation
    serde_json::to_string(&vec![
        json!({"id": 1, "make": &make, "model": "Camry", "price": 25000}),
        json!({"id": 2, "make": &make, "model": "Corolla", "price": 22000}),
    ]).unwrap()
}

#[derive(Deserialize)]
struct ChatRequest {
    message: String,
    thread_id: String,
}

async fn chat_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<ChatRequest>,
) -> Json<serde_json::Value> {
    let response = state.agent.handle_message(
        &req.message,
        Arc::new(AgentStateSnapshot::default())
    ).await.unwrap();
    
    Json(json!({
        "response": response.content.as_text(),
        "tool_calls": response.tool_calls.len(),
    }))
}

async fn events_handler(
    State(state): State<Arc<AppState>>,
) -> Sse<impl futures::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let mut rx = state.event_tx.subscribe();
    
    let stream = async_stream::stream! {
        while let Ok(event) = rx.recv().await {
            let json = serde_json::to_string(&event).unwrap();
            yield Ok(Event::default().data(json));
        }
    };
    
    Sse::new(stream)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let api_key = std::env::var("OPENAI_API_KEY")?;
    
    let model = Arc::new(OpenAiChatModel::new(
        OpenAiConfig::new(api_key, "gpt-4o-mini")
    )?);
    
    let (event_tx, _) = broadcast::channel(100);
    let dispatcher = Arc::new(EventDispatcher::from_sender(event_tx.clone()));
    
    let agent = ConfigurableAgentBuilder::new(
        "You are an automotive sales assistant."
    )
    .with_model(model)
    .with_tool(SearchVehiclesTool::as_tool())
    .with_event_dispatcher(dispatcher)
    .build()?;
    
    let state = Arc::new(AppState {
        agent: Arc::new(agent),
        event_tx,
    });
    
    let app = Router::new()
        .route("/api/chat", post(chat_handler))
        .route("/api/events", get(events_handler))
        .with_state(state);
    
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;
    axum::serve(listener, app).await?;
    
    Ok(())
}
```

## Frontend Code

```typescript
// src/App.tsx
import { useState, useEffect } from 'react';

function App() {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState('');
  const [streaming, setStreaming] = useState('');

  useEffect(() => {
    const eventSource = new EventSource('/api/events');
    
    eventSource.onmessage = (e) => {
      const event = JSON.parse(e.data);
      
      if (event.event_type === 'streaming_token') {
        setStreaming(prev => prev + event.content);
      } else if (event.event_type === 'agent_completed') {
        setMessages(prev => [...prev, { 
          role: 'assistant', 
          content: streaming 
        }]);
        setStreaming('');
      }
    };
    
    return () => eventSource.close();
  }, []);

  const sendMessage = async () => {
    setMessages(prev => [...prev, { role: 'user', content: input }]);
    
    await fetch('/api/chat', {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message: input, thread_id: 'user-1' }),
    });
    
    setInput('');
  };

  return (
    <div className="chat-container">
      <div className="messages">
        {messages.map((m, i) => (
          <div key={i} className={`message ${m.role}`}>
            {m.content}
          </div>
        ))}
        {streaming && (
          <div className="message assistant streaming">
            {streaming}
          </div>
        )}
      </div>
      <input 
        value={input}
        onChange={e => setInput(e.target.value)}
        onKeyPress={e => e.key === 'Enter' && sendMessage()}
      />
    </div>
  );
}
```

## Run It

```bash
cd examples/automotive-web-service

# Backend
export OPENAI_API_KEY="your-key"
cargo run

# Frontend (new terminal)
cd frontend
npm install
npm run dev
```

Open `http://localhost:5173`

## What It Demonstrates

- Axum web server setup
- SSE streaming implementation
- React real-time updates
- Tool integration
- Production architecture patterns

