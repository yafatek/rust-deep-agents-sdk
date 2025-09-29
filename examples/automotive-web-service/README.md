# Automotive Web Service

A full-stack web application demonstrating the Rust Deep Agents SDK with Server-Sent Events (SSE) streaming and a modern React TypeScript frontend.

## Features

### Backend (Rust + Axum)
- **SSE Streaming**: Real-time streaming responses from the agent
- **Checkpointing**: Conversation state persistence across sessions
- **Summarization**: Automatic context optimization (keeps last 10 messages)
- **6 Specialized Sub-Agents**:
  - 🔧 Diagnostic Agent - Vehicle issue analysis
  - 📅 Booking Agent - Service scheduling
  - 🎫 Ticketing Agent - Support ticket management
  - 💳 Payment Agent - Billing and invoices
  - 🔔 Notification Agent - Customer notifications
  - ⭐ Feedback Agent - Customer feedback collection

### Frontend (React + TypeScript + Tailwind + shadcn/ui)
- **Real-time Chat Interface**: Token-by-token streaming display
- **Agent Activity Monitor**: Live visualization of sub-agent activities
- **Feature Panel**: Display of active agentic features (checkpointing, summarization, sub-agents)
- **Responsive Design**: Mobile-friendly interface with collapsible sidebar
- **Dark Mode Support**: Built-in dark mode styling

## Architecture

```
┌─────────────────────┐         SSE          ┌──────────────────────┐
│   React Frontend    │◄─────────────────────┤   Axum Backend       │
│   (Port 5173)       │                      │   (Port 3001)        │
│                     │                      │                      │
│  - Chat UI          │    HTTP POST         │  - SSE Endpoint      │
│  - Agent Activity   ├──────────────────────►  - Agent Runtime     │
│  - Feature Panel    │                      │  - 6 Sub-Agents      │
└─────────────────────┘                      └──────────────────────┘
```

## Getting Started

### Prerequisites
- Rust (1.70+)
- Node.js (18+)
- OpenAI API Key

### Setup

1. **Set OpenAI API Key**:
```bash
export OPENAI_API_KEY=your_api_key_here
```

2. **Start the Backend**:
```bash
cd rust-deep-agents
cargo run -p automotive-web-service
```

The backend will start on `http://0.0.0.0:3001`

3. **Start the Frontend** (in a new terminal):
```bash
cd rust-deep-agents/examples/automotive-web-service/frontend
npm install  # First time only
npm run dev
```

The frontend will start on `http://localhost:5173`

4. **Open your browser** and navigate to `http://localhost:5173`

## Usage

### Example Prompts

Try asking the assistant:
- "My car is making a strange noise when I brake"
- "I need to book a service appointment for next week"
- "What's the status of my service ticket?"
- "I want to pay my invoice"
- "Can you send me a reminder about my upcoming service?"

### Features in Action

**Streaming**: Watch as responses appear token-by-token in real-time

**Sub-Agent Delegation**: The coordinator agent intelligently delegates tasks to specialized sub-agents based on your request

**Checkpointing**: Your conversation is automatically saved. Refresh the page and continue where you left off using the same session ID

**Summarization**: When conversations get long (>10 messages), earlier context is automatically summarized to optimize token usage

## API Endpoints

### Backend Endpoints

- **GET /health**: Health check endpoint
- **GET /chat/stream**: SSE streaming endpoint
  - Query params:
    - `message` (required): User message
    - `session_id` (optional): Session ID for conversation continuation
- **GET /sessions**: List active sessions

### SSE Event Types

The backend sends the following SSE events:

1. **session**: Contains the session ID
```json
{"session_id": "uuid-here"}
```

2. **delta**: Text chunks as they arrive
```json
{"text": "partial response text"}
```

3. **done**: Final complete message
```json
{"text": "complete response text"}
```

4. **error**: Error information
```json
{"error": "error message"}
```

## Project Structure

```
automotive-web-service/
├── src/
│   └── main.rs           # Rust backend with Axum and SSE
├── frontend/
│   ├── src/
│   │   ├── components/   # React components
│   │   │   ├── EnhancedChat.tsx
│   │   │   ├── ChatMessage.tsx
│   │   │   ├── ChatInput.tsx
│   │   │   ├── AgentActivity.tsx
│   │   │   └── FeaturePanel.tsx
│   │   ├── hooks/
│   │   │   └── useSSE.ts  # SSE connection hook
│   │   ├── lib/
│   │   │   └── utils.ts   # Utility functions
│   │   ├── App.tsx
│   │   └── main.tsx
│   ├── package.json
│   └── tailwind.config.js
└── Cargo.toml
```

## Development

### Backend Development
```bash
cargo watch -x 'run -p automotive-web-service'
```

### Frontend Development
```bash
cd frontend
npm run dev
```

### Build for Production

**Backend**:
```bash
cargo build --release -p automotive-web-service
```

**Frontend**:
```bash
cd frontend
npm run build
```

## Technologies Used

### Backend
- **Rust** - Systems programming language
- **Axum** - Web framework
- **Tower** - Middleware and utilities
- **Tower-HTTP** - HTTP-specific middleware (CORS, static files)
- **Tokio** - Async runtime
- **agents-sdk** - Deep agents framework

### Frontend
- **React** - UI library
- **TypeScript** - Type-safe JavaScript
- **Vite** - Build tool and dev server
- **Tailwind CSS** - Utility-first CSS framework
- **Radix UI** - Accessible component primitives
- **lucide-react** - Icon library
- **clsx + tailwind-merge** - Conditional class management

## License

Apache 2.0

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.