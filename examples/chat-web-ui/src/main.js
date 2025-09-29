// Deep Agent Chat Web UI
// Modern chat interface for the Deep Agent HTTP Server

import axios from 'axios';
import { v4 as uuidv4 } from 'uuid';

class DeepAgentChat {
    constructor() {
        this.apiUrl = 'http://localhost:3000/api/v1';
        this.sessionId = null;
        this.isTyping = false;
        this.statusPollingInterval = null;
        
        this.initializeElements();
        this.setupEventListeners();
        this.checkServerHealth();
        
        // Auto-resize textarea
        this.setupTextareaResize();
        
        console.log('🧠 Deep Agent Chat initialized');
    }
    
    initializeElements() {
        this.messagesContainer = document.getElementById('messages');
        this.messageInput = document.getElementById('message-input');
        this.sendButton = document.getElementById('send-button');
        this.agentTypeSelect = document.getElementById('agent-type');
        this.statusText = document.getElementById('status-text');
        this.sessionIdSpan = document.getElementById('session-id');
        
        // Progress panel elements
        this.agentStatusBadge = document.getElementById('agent-status-badge');
        this.currentTaskDiv = document.getElementById('current-task');
        this.todoItemsDiv = document.getElementById('todo-items');
        this.actionItemsDiv = document.getElementById('action-items');
    }
    
    setupEventListeners() {
        // Send button click
        this.sendButton.addEventListener('click', () => this.sendMessage());
        
        // Enter key to send (Shift+Enter for new line)
        this.messageInput.addEventListener('keydown', (e) => {
            if (e.key === 'Enter' && !e.shiftKey) {
                e.preventDefault();
                this.sendMessage();
            }
        });
        
        // Auto-focus input
        this.messageInput.focus();
    }
    
    setupTextareaResize() {
        this.messageInput.addEventListener('input', () => {
            this.messageInput.style.height = 'auto';
            this.messageInput.style.height = Math.min(this.messageInput.scrollHeight, 120) + 'px';
        });
    }
    
    async checkServerHealth() {
        try {
            const response = await axios.get(`${this.apiUrl}/health`);
            this.updateStatus('Connected', 'healthy');
            console.log('✅ Server health:', response.data);
        } catch (error) {
            this.updateStatus('Disconnected', 'error');
            this.addMessage('system', '❌ Cannot connect to Deep Agent server. Make sure it\'s running on localhost:3000');
            console.error('❌ Server health check failed:', error);
        }
    }
    
    updateStatus(text, type = 'healthy') {
        this.statusText.textContent = text;
        const statusDot = document.querySelector('.status-dot');
        
        if (type === 'error') {
            statusDot.style.background = '#ef4444';
        } else if (type === 'thinking') {
            statusDot.style.background = '#f59e0b';
        } else {
            statusDot.style.background = '#10b981';
        }
    }
    
    async sendMessage() {
        const message = this.messageInput.value.trim();
        if (!message || this.isTyping) return;
        
        // Add user message to chat
        this.addMessage('user', message);
        
        // Clear input and disable send button
        this.messageInput.value = '';
        this.messageInput.style.height = 'auto';
        this.setSendingState(true);
        
        try {
            // Show typing indicator
            this.showTypingIndicator();
            this.updateStatus('Thinking...', 'thinking');
            
            // Send to API
            const response = await axios.post(`${this.apiUrl}/chat`, {
                message: message,
                session_id: this.sessionId,
                agent_type: this.agentTypeSelect.value
            });
            
            // Update session ID if new
            if (!this.sessionId) {
                this.sessionId = response.data.session_id;
                this.sessionIdSpan.textContent = this.sessionId.substring(0, 8) + '...';
                this.startStatusPolling();
            }
            
            // Hide typing indicator and add response
            this.hideTypingIndicator();
            this.addMessage('agent', response.data.response, {
                timestamp: response.data.timestamp,
                sessionId: response.data.session_id
            });
            
            this.updateStatus('Connected', 'healthy');
            
        } catch (error) {
            this.hideTypingIndicator();
            this.updateStatus('Error', 'error');
            
            let errorMessage = 'Failed to send message. ';
            if (error.response) {
                errorMessage += `Server error: ${error.response.status}`;
                if (error.response.data?.error) {
                    errorMessage += ` - ${error.response.data.error}`;
                }
            } else if (error.request) {
                errorMessage += 'Cannot reach server. Make sure it\'s running.';
            } else {
                errorMessage += error.message;
            }
            
            this.addMessage('error', errorMessage);
            console.error('❌ Send message failed:', error);
        } finally {
            this.setSendingState(false);
            this.messageInput.focus();
        }
    }
    
    addMessage(type, content, meta = {}) {
        const messageDiv = document.createElement('div');
        messageDiv.className = `message ${type}`;
        
        // Format content for better display
        let formattedContent = content;
        if (type === 'agent') {
            // Convert markdown-like formatting to HTML
            formattedContent = content
                .replace(/\*\*(.*?)\*\*/g, '<strong>$1</strong>')
                .replace(/\*(.*?)\*/g, '<em>$1</em>')
                .replace(/`(.*?)`/g, '<code style="background: #f1f5f9; padding: 0.125rem 0.25rem; border-radius: 0.25rem; font-family: monospace;">$1</code>')
                .replace(/\n/g, '<br>');
        }
        
        messageDiv.innerHTML = formattedContent;
        
        // Add metadata if available
        if (meta.timestamp) {
            const metaDiv = document.createElement('div');
            metaDiv.className = 'message-meta';
            metaDiv.textContent = new Date(meta.timestamp).toLocaleTimeString();
            messageDiv.appendChild(metaDiv);
        }
        
        this.messagesContainer.appendChild(messageDiv);
        this.scrollToBottom();
    }
    
    showTypingIndicator() {
        if (this.typingIndicator) return;
        
        this.typingIndicator = document.createElement('div');
        this.typingIndicator.className = 'typing-indicator';
        this.typingIndicator.innerHTML = `
            <span>Deep Agent is thinking</span>
            <div class="typing-dots">
                <div class="typing-dot"></div>
                <div class="typing-dot"></div>
                <div class="typing-dot"></div>
            </div>
        `;
        
        this.messagesContainer.appendChild(this.typingIndicator);
        this.scrollToBottom();
        this.isTyping = true;
    }
    
    hideTypingIndicator() {
        if (this.typingIndicator) {
            this.typingIndicator.remove();
            this.typingIndicator = null;
        }
        this.isTyping = false;
    }
    
    setSendingState(sending) {
        this.sendButton.disabled = sending;
        if (sending) {
            this.sendButton.innerHTML = '<span>Sending...</span><span>⏳</span>';
        } else {
            this.sendButton.innerHTML = '<span>Send</span><span>📤</span>';
        }
    }
    
    scrollToBottom() {
        this.messagesContainer.scrollTop = this.messagesContainer.scrollHeight;
    }
    
    startStatusPolling() {
        if (this.statusPollingInterval) {
            clearInterval(this.statusPollingInterval);
        }
        
        // Poll every 1 second for real-time updates
        this.statusPollingInterval = setInterval(() => {
            this.updateAgentStatus();
        }, 1000);
        
        // Initial status update
        this.updateAgentStatus();
    }
    
    async updateAgentStatus() {
        if (!this.sessionId) return;
        
        try {
            const response = await axios.get(`${this.apiUrl}/status/${this.sessionId}`);
            const status = response.data;
            
            this.updateStatusDisplay(status);
        } catch (error) {
            console.warn('Failed to fetch agent status:', error);
        }
    }
    
    updateStatusDisplay(status) {
        // Update status badge
        this.agentStatusBadge.className = `status-badge status-${status.status}`;
        this.agentStatusBadge.textContent = this.capitalizeFirst(status.status);
        
        // Update current task
        this.currentTaskDiv.textContent = status.current_task || 'No active task';
        
        // Update todos
        this.updateTodoList(status.todos);
        
        // Update recent actions
        this.updateActionsList(status.recent_actions);
        
        // Update active subagent if any
        if (status.active_subagent) {
            this.currentTaskDiv.innerHTML += ` <span style="color: #8b5cf6;">(${status.active_subagent})</span>`;
        }
    }
    
    updateTodoList(todos) {
        if (!todos || todos.length === 0) {
            this.todoItemsDiv.innerHTML = `
                <div class="todo-item todo-pending">
                    <span>📝</span>
                    <span>Waiting for user input...</span>
                </div>
            `;
            return;
        }
        
        this.todoItemsDiv.innerHTML = todos.map(todo => {
            const icon = todo.status === 'completed' ? '✅' : 
                        todo.status === 'in_progress' ? '🔄' : '📝';
            return `
                <div class="todo-item todo-${todo.status}">
                    <span>${icon}</span>
                    <span>${todo.content}</span>
                </div>
            `;
        }).join('');
    }
    
    updateActionsList(actions) {
        if (!actions || actions.length === 0) {
            this.actionItemsDiv.innerHTML = `
                <div class="action-item">
                    <div>No recent actions</div>
                    <div style="font-size: 0.6rem; opacity: 0.7;">Waiting...</div>
                </div>
            `;
            return;
        }
        
        // Show last 5 actions
        const recentActions = actions.slice(-5).reverse();
        this.actionItemsDiv.innerHTML = recentActions.map(action => {
            const timeAgo = this.getTimeAgo(new Date(action.timestamp));
            return `
                <div class="action-item action-${action.action_type.replace('_', '-')}">
                    <div>${action.description}</div>
                    <div style="font-size: 0.6rem; opacity: 0.7;">${timeAgo}</div>
                </div>
            `;
        }).join('');
    }
    
    capitalizeFirst(str) {
        return str.charAt(0).toUpperCase() + str.slice(1).replace('_', ' ');
    }
    
    getTimeAgo(date) {
        const now = new Date();
        const diffMs = now - date;
        const diffSecs = Math.floor(diffMs / 1000);
        const diffMins = Math.floor(diffSecs / 60);
        
        if (diffSecs < 60) return 'Just now';
        if (diffMins < 60) return `${diffMins}m ago`;
        return `${Math.floor(diffMins / 60)}h ago`;
    }
}

// Initialize the chat when DOM is loaded
document.addEventListener('DOMContentLoaded', () => {
    window.deepAgentChat = new DeepAgentChat();
});

// Add some example prompts for testing
window.examplePrompts = [
    "What is quantum computing and how does it work?",
    "Research the latest developments in artificial intelligence",
    "Compare renewable energy sources: solar vs wind",
    "Explain blockchain technology and its applications",
    "What are the current trends in space exploration?",
    "Analyze the impact of remote work on productivity"
];

// Add keyboard shortcut for example prompts (Ctrl/Cmd + E)
document.addEventListener('keydown', (e) => {
    if ((e.ctrlKey || e.metaKey) && e.key === 'e') {
        e.preventDefault();
        const randomPrompt = window.examplePrompts[Math.floor(Math.random() * window.examplePrompts.length)];
        document.getElementById('message-input').value = randomPrompt;
        document.getElementById('message-input').focus();
    }
});

console.log('💡 Tip: Press Ctrl/Cmd + E for example prompts!');
