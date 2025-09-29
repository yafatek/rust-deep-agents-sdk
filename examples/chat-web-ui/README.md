# Deep Agent Chat Web UI

A beautiful, modern web chat interface for testing the Deep Agent HTTP Server. Built with Vite.js for fast development and smooth user experience.

## 🌟 Features

### 💬 **Modern Chat Interface**
- Beautiful gradient design with glassmorphism effects
- Smooth animations and transitions
- Responsive design for desktop and mobile
- Real-time typing indicators
- Message timestamps and metadata

### 🤖 **Deep Agent Integration**
- Direct connection to Deep Agent HTTP Server
- Session management with persistent conversations
- Agent type selection (research, general, etc.)
- Real-time server health monitoring
- Error handling with user-friendly messages

### ⚡ **Developer Experience**
- Hot module replacement with Vite
- Modern ES6+ JavaScript
- Axios for HTTP requests
- UUID for session management
- Clean, maintainable code structure

### 🎨 **UI/UX Features**
- Auto-resizing message input
- Keyboard shortcuts (Enter to send, Shift+Enter for new line)
- Example prompts (Ctrl/Cmd + E)
- Smooth scrolling to new messages
- Visual status indicators

## 🚀 Quick Start

### Prerequisites
- Node.js 16+ installed
- Deep Agent HTTP Server running on `localhost:3000`

### Setup
```bash
cd examples/chat-web-ui
npm install
npm run dev
```

The web interface will be available at `http://localhost:5173`

### Build for Production
```bash
npm run build
npm run preview
```

## 🔧 Configuration

### Server URL
The default API URL is `http://localhost:3000/api/v1`. To change this, edit `src/main.js`:

```javascript
this.apiUrl = 'http://your-server:port/api/v1';
```

### Agent Types
Add more agent types by editing the select options in `index.html`:

```html
<select class="agent-selector" id="agent-type">
    <option value="research">🔬 Research Agent</option>
    <option value="creative">🎨 Creative Agent</option>
    <option value="technical">⚙️ Technical Agent</option>
</select>
```

## 🎯 Usage Examples

### Basic Chat
1. Open the web interface
2. Type your message in the input field
3. Press Enter or click Send
4. Watch the Deep Agent respond with intelligent analysis

### Research Queries
Try these example prompts:
- "What is quantum computing and how does it work?"
- "Research the latest developments in AI"
- "Compare solar vs wind energy"
- "Analyze blockchain technology applications"

### Keyboard Shortcuts
- **Enter**: Send message
- **Shift + Enter**: New line in message
- **Ctrl/Cmd + E**: Insert random example prompt

## 🏗️ Architecture

```
Web UI (Vite.js)
├── index.html          → Main HTML structure
├── src/main.js         → Chat logic and API integration
├── package.json        → Dependencies and scripts
└── vite.config.js      → Vite configuration

HTTP API Integration
├── Health Check        → GET /api/v1/health
├── Chat Messages       → POST /api/v1/chat
├── Session Management  → GET /api/v1/sessions
└── Agent Info          → GET /api/v1/agents
```

## 🎨 Customization

### Styling
The interface uses CSS custom properties for easy theming. Main colors:

```css
:root {
  --primary-gradient: linear-gradient(135deg, #4f46e5 0%, #7c3aed 100%);
  --background-gradient: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
  --message-user-bg: var(--primary-gradient);
  --message-agent-bg: #f8fafc;
}
```

### Adding Features
The modular JavaScript structure makes it easy to add features:

```javascript
class DeepAgentChat {
    // Add new methods here
    async uploadFile() { /* file upload logic */ }
    exportChat() { /* export conversation */ }
    clearHistory() { /* clear chat history */ }
}
```

## 🔍 Testing

### Manual Testing
1. **Connection Test**: Check if status shows "Connected"
2. **Basic Chat**: Send "Hello" and verify response
3. **Research Test**: Ask a complex research question
4. **Error Handling**: Stop the server and test error messages
5. **Session Persistence**: Refresh page and continue conversation

### Example Test Scenarios
```javascript
// Test prompts for different capabilities
const testPrompts = [
    "Hello, what can you help me with?",                    // Basic interaction
    "Research quantum computing applications",               // Research capability
    "Compare pros and cons of different programming languages", // Analysis
    "What are the latest trends in renewable energy?",      // Current information
];
```

## 🚀 Deployment

### Static Hosting
```bash
npm run build
# Deploy the 'dist' folder to any static hosting service
```

### Docker
```dockerfile
FROM node:18-alpine as builder
WORKDIR /app
COPY package*.json ./
RUN npm install
COPY . .
RUN npm run build

FROM nginx:alpine
COPY --from=builder /app/dist /usr/share/nginx/html
EXPOSE 80
```

### Environment Variables
For production deployment, consider using environment variables:

```javascript
const API_URL = import.meta.env.VITE_API_URL || 'http://localhost:3000/api/v1';
```

## 🐛 Troubleshooting

### Common Issues

**"Cannot connect to server"**
- Ensure Deep Agent HTTP Server is running on port 3000
- Check CORS settings in the server
- Verify network connectivity

**"Messages not sending"**
- Check browser console for JavaScript errors
- Verify API endpoint URLs
- Test server health endpoint directly

**"Styling issues"**
- Clear browser cache
- Check for CSS conflicts
- Verify Vite build process

### Debug Mode
Enable debug logging by adding to `src/main.js`:

```javascript
const DEBUG = true;
if (DEBUG) {
    console.log('Debug info:', data);
}
```

## 🤝 Integration

This web UI can be easily integrated into larger applications:

### React Integration
```jsx
import { useEffect } from 'react';

function ChatComponent() {
    useEffect(() => {
        // Initialize Deep Agent Chat
        const chat = new DeepAgentChat();
        return () => chat.cleanup();
    }, []);
    
    return <div id="deep-agent-chat"></div>;
}
```

### Vue Integration
```vue
<template>
    <div id="deep-agent-chat"></div>
</template>

<script>
import { DeepAgentChat } from './deep-agent-chat.js';

export default {
    mounted() {
        this.chat = new DeepAgentChat();
    },
    beforeUnmount() {
        this.chat?.cleanup();
    }
}
</script>
```

Perfect for testing, development, and production use! 🎉
