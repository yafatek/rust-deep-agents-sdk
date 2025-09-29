# Deep Agent HTTP Server

A **production-ready web service** that exposes Deep Agent functionality via REST API. Perfect for integrating AI agents into web applications, mobile apps, or microservice architectures.

## 🚀 Features

### 🌐 **HTTP REST API**
- RESTful endpoints for agent interactions
- JSON request/response format
- CORS enabled for web applications
- Comprehensive error handling

### 🔄 **Session Management**
- Persistent sessions with unique IDs
- Session tracking and analytics
- Automatic session creation
- Session history and metadata

### 🤖 **Deep Agent Integration**
- Full Deep Agent capabilities via HTTP
- Multiple specialized subagents
- Real-time web search via Tavily
- File system operations
- Planning and task delegation

### 📊 **Production Ready**
- Health checks and monitoring
- Graceful shutdown handling
- Structured logging with tracing
- Performance optimized with Tokio

## 📡 API Endpoints

### `POST /api/v1/chat`
Send a message to the agent and get a response.

**Request:**
```json
{
  "message": "What is quantum computing?",
  "session_id": "optional-session-id",
  "agent_type": "research"
}
```

**Response:**
```json
{
  "response": "Quantum computing is a revolutionary computing paradigm...",
  "session_id": "uuid-session-id",
  "timestamp": "2024-01-15T10:30:00Z",
  "files_created": ["research_report.md"],
  "tools_used": ["internet_search", "task"]
}
```

### `GET /api/v1/sessions/{id}`
Get information about a specific session.

**Response:**
```json
{
  "id": "session-uuid",
  "created_at": "2024-01-15T10:00:00Z",
  "last_activity": "2024-01-15T10:30:00Z",
  "message_count": 5,
  "agent_type": "research"
}
```

### `GET /api/v1/sessions`
List all active sessions.

### `GET /api/v1/health`
Health check endpoint for monitoring.

**Response:**
```json
{
  "status": "healthy",
  "version": "0.1.0",
  "uptime_seconds": 3600,
  "active_sessions": 12
}
```

### `GET /api/v1/agents`
List available agents and their capabilities.

**Response:**
```json
[
  {
    "name": "research",
    "description": "Deep research agent with specialized subagents",
    "tools": ["internet_search", "write_file", "task"],
    "subagents": ["research-agent", "critique-agent"]
  }
]
```

## 🛠️ Usage

### Setup
```bash
cd examples/deep-agent-server
cp .env.example .env
# Edit .env with your API keys
```

### Run the Server
```bash
# Default (port 3000)
cargo run

# Custom port and host
cargo run -- --port 8080 --host 127.0.0.1

# Verbose logging
cargo run -- --verbose
```

### Test the API
```bash
# Health check
curl http://localhost:3000/api/v1/health

# Send a message
curl -X POST http://localhost:3000/api/v1/chat \
  -H 'Content-Type: application/json' \
  -d '{
    "message": "Research the latest developments in quantum computing",
    "agent_type": "research"
  }'

# Get session info
curl http://localhost:3000/api/v1/sessions/your-session-id

# List all sessions
curl http://localhost:3000/api/v1/sessions

# Get agent capabilities
curl http://localhost:3000/api/v1/agents
```

## 🔧 Integration Examples

### JavaScript/TypeScript
```typescript
class DeepAgentClient {
  constructor(private baseUrl: string = 'http://localhost:3000/api/v1') {}

  async chat(message: string, sessionId?: string): Promise<ChatResponse> {
    const response = await fetch(`${this.baseUrl}/chat`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ message, session_id: sessionId })
    });
    return response.json();
  }

  async getSession(sessionId: string): Promise<SessionInfo> {
    const response = await fetch(`${this.baseUrl}/sessions/${sessionId}`);
    return response.json();
  }
}

// Usage
const client = new DeepAgentClient();
const result = await client.chat("What is quantum computing?");
console.log(result.response);
```

### Python
```python
import requests

class DeepAgentClient:
    def __init__(self, base_url="http://localhost:3000/api/v1"):
        self.base_url = base_url
    
    def chat(self, message: str, session_id: str = None) -> dict:
        response = requests.post(f"{self.base_url}/chat", json={
            "message": message,
            "session_id": session_id
        })
        return response.json()
    
    def get_session(self, session_id: str) -> dict:
        response = requests.get(f"{self.base_url}/sessions/{session_id}")
        return response.json()

# Usage
client = DeepAgentClient()
result = client.chat("Research quantum computing applications")
print(result["response"])
```

### cURL Examples
```bash
# Simple chat
curl -X POST http://localhost:3000/api/v1/chat \
  -H 'Content-Type: application/json' \
  -d '{"message": "Explain machine learning"}'

# Chat with session
curl -X POST http://localhost:3000/api/v1/chat \
  -H 'Content-Type: application/json' \
  -d '{
    "message": "Continue our previous discussion",
    "session_id": "my-session-123"
  }'

# Research request
curl -X POST http://localhost:3000/api/v1/chat \
  -H 'Content-Type: application/json' \
  -d '{
    "message": "Research and compare solar vs wind energy",
    "agent_type": "research"
  }'
```

## 🏗️ Architecture

```
HTTP Server (Axum)
├── API Routes
│   ├── /api/v1/chat          → Chat Handler
│   ├── /api/v1/sessions/*    → Session Management
│   ├── /api/v1/health        → Health Checks
│   └── /api/v1/agents        → Agent Info
├── Deep Agent
│   ├── Main Agent (Research Orchestrator)
│   ├── Research Subagent (Specialized Research)
│   ├── Critique Subagent (Quality Review)
│   └── Tools: internet_search, file_ops, task
├── Session Management
│   ├── In-Memory Session Store
│   ├── Session Tracking
│   └── Automatic Cleanup
└── External APIs
    ├── OpenAI (GPT-4o-mini)
    └── Tavily (Web Search)
```

## 🚀 Production Deployment

### Docker
```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin deep-agent-server

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/deep-agent-server /usr/local/bin/
EXPOSE 3000
CMD ["deep-agent-server"]
```

### Environment Variables
```bash
# Required
OPENAI_API_KEY=your_key_here
TAVILY_API_KEY=your_key_here

# Optional
RUST_LOG=info
SERVER_HOST=0.0.0.0
SERVER_PORT=3000
```

### Monitoring
- Health endpoint: `GET /api/v1/health`
- Structured logging with tracing
- Session metrics and analytics
- Graceful shutdown handling

## 🔒 Security Considerations

- **API Keys**: Store securely in environment variables
- **CORS**: Configure appropriately for your domain
- **Rate Limiting**: Add rate limiting middleware for production
- **Authentication**: Add JWT or API key authentication as needed
- **Input Validation**: All inputs are validated and sanitized

## 🎯 Use Cases

- **Web Applications**: Integrate AI research capabilities
- **Mobile Apps**: Backend API for AI-powered features
- **Microservices**: AI agent as a service in your architecture
- **Chatbots**: Power conversational AI with deep research
- **Content Generation**: Automated research and report generation
- **API Gateway**: Expose AI capabilities to multiple clients

This HTTP server transforms your Deep Agent into a **production-ready web service** that can be integrated into any application or service! 🌐
