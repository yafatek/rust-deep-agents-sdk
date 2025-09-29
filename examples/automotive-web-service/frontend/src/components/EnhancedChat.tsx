import { useEffect, useRef, useState } from 'react';
import { useSSE } from '../hooks/useSSE';
import { ChatMessage } from './ChatMessage';
import { ChatInput } from './ChatInput';
import { AgentActivity } from './AgentActivity';
import type { AgentActivityItem } from '../types';
import { FeaturePanel } from './FeaturePanel';
import { Bot, Loader2, Menu, X, Sparkles } from 'lucide-react';
import { cn } from '../lib/utils';

const BACKEND_URL = import.meta.env.VITE_BACKEND_URL || 'http://localhost:3001';

export function EnhancedChat() {
  const { messages, sendMessage, isStreaming, currentStreamingMessage, sessionId } = useSSE(BACKEND_URL);
  const messagesEndRef = useRef<HTMLDivElement>(null);
  const [activities, setActivities] = useState<AgentActivityItem[]>([]);
  const [sidebarOpen, setSidebarOpen] = useState(true);

  // Auto-scroll to bottom when new messages arrive
  useEffect(() => {
    messagesEndRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [messages, currentStreamingMessage]);

  // Simulate agent activities for demonstration
  useEffect(() => {
    if (isStreaming) {
      const activity: AgentActivityItem = {
        agent: 'Coordinator',
        action: 'Processing request...',
        timestamp: new Date(),
        status: 'active',
      };
      setActivities(prev => [...prev, activity]);

      // Simulate sub-agent activities
      const timeout = setTimeout(() => {
        const subAgents = ['Diagnostic', 'Booking', 'Ticketing', 'Payment', 'Notification'];
        const randomAgent = subAgents[Math.floor(Math.random() * subAgents.length)];
        const activity: AgentActivityItem = {
          agent: randomAgent,
          action: 'Analyzing...',
          timestamp: new Date(),
          status: 'active',
        };
        setActivities(prev => [...prev, activity]);
      }, 1000);

      return () => clearTimeout(timeout);
    }
  }, [isStreaming]);

  return (
    <div className="flex h-screen">
      {/* Main Chat Area */}
      <div className="flex flex-col flex-1 min-w-0">
        {/* Header */}
        <div className="flex items-center gap-4 px-6 py-4 bg-white/80 dark:bg-slate-900/80 backdrop-blur-xl border-b border-slate-200 dark:border-slate-800 shadow-sm">
          <button
            onClick={() => setSidebarOpen(!sidebarOpen)}
            className="p-2 hover:bg-slate-100 dark:hover:bg-slate-800 rounded-xl transition-all duration-200"
          >
            {sidebarOpen ? <X size={20} /> : <Menu size={20} />}
          </button>

          <div className="flex items-center justify-center w-12 h-12 rounded-2xl bg-gradient-to-br from-blue-500 to-purple-600 shadow-lg shadow-blue-500/20">
            <Bot className="w-6 h-6 text-white" />
          </div>

          <div className="flex-1 min-w-0">
            <h1 className="text-lg font-bold text-slate-900 dark:text-white flex items-center gap-2">
              Automotive AI Assistant
              <Sparkles className="w-4 h-4 text-yellow-500" />
            </h1>
            <p className="text-sm text-slate-500 dark:text-slate-400 truncate">
              {sessionId ? `Session: ${sessionId.substring(0, 8)}...` : 'Powered by 6 specialized AI agents'}
            </p>
          </div>

          {isStreaming && (
            <div className="flex items-center gap-2 px-4 py-2 bg-blue-50 dark:bg-blue-950/30 rounded-full">
              <Loader2 className="w-4 h-4 animate-spin text-blue-600 dark:text-blue-400" />
              <span className="text-sm font-medium text-blue-600 dark:text-blue-400">Thinking...</span>
            </div>
          )}
        </div>

        {/* Messages */}
        <div className="flex-1 overflow-y-auto px-6 py-8">
          <div className="max-w-4xl mx-auto space-y-6">
            {messages.length === 0 && (
              <div className="flex flex-col items-center justify-center min-h-[60vh] text-center">
                <div className="relative mb-8">
                  <div className="absolute inset-0 bg-gradient-to-r from-blue-500 to-purple-600 rounded-3xl blur-2xl opacity-20"></div>
                  <div className="relative flex items-center justify-center w-24 h-24 rounded-3xl bg-gradient-to-br from-blue-500 to-purple-600 shadow-2xl">
                    <Bot className="w-12 h-12 text-white" />
                  </div>
                </div>

                <h2 className="text-3xl font-bold mb-3 bg-gradient-to-r from-slate-900 to-slate-700 dark:from-white dark:to-slate-300 bg-clip-text text-transparent">
                  Welcome to Automotive AI
                </h2>

                <p className="text-slate-600 dark:text-slate-400 max-w-md mb-8 text-lg">
                  Your intelligent assistant powered by specialized AI agents
                </p>

                <div className="grid grid-cols-2 gap-4 max-w-2xl">
                  <div className="group p-6 rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:shadow-lg hover:scale-105 transition-all duration-200">
                    <div className="text-4xl mb-3">🔧</div>
                    <p className="font-semibold text-slate-900 dark:text-white mb-1">Diagnostics</p>
                    <p className="text-sm text-slate-500 dark:text-slate-400">AI-powered vehicle analysis</p>
                  </div>

                  <div className="group p-6 rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:shadow-lg hover:scale-105 transition-all duration-200">
                    <div className="text-4xl mb-3">📅</div>
                    <p className="font-semibold text-slate-900 dark:text-white mb-1">Bookings</p>
                    <p className="text-sm text-slate-500 dark:text-slate-400">Smart scheduling system</p>
                  </div>

                  <div className="group p-6 rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:shadow-lg hover:scale-105 transition-all duration-200">
                    <div className="text-4xl mb-3">🎫</div>
                    <p className="font-semibold text-slate-900 dark:text-white mb-1">Support</p>
                    <p className="text-sm text-slate-500 dark:text-slate-400">Ticket management</p>
                  </div>

                  <div className="group p-6 rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 hover:shadow-lg hover:scale-105 transition-all duration-200">
                    <div className="text-4xl mb-3">💳</div>
                    <p className="font-semibold text-slate-900 dark:text-white mb-1">Payments</p>
                    <p className="text-sm text-slate-500 dark:text-slate-400">Secure billing</p>
                  </div>
                </div>
              </div>
            )}

            {messages.map((message, index) => (
              <ChatMessage key={index} message={message} />
            ))}

            {currentStreamingMessage && (
              <ChatMessage
                message={{
                  role: 'assistant',
                  content: currentStreamingMessage,
                  timestamp: new Date(),
                }}
                isStreaming={true}
              />
            )}

            <div ref={messagesEndRef} />
          </div>
        </div>

        {/* Input */}
        <ChatInput onSend={sendMessage} disabled={isStreaming} />
      </div>

      {/* Sidebar */}
      <div
        className={cn(
          'border-l border-slate-200 dark:border-slate-800 bg-white/50 dark:bg-slate-900/50 backdrop-blur-xl transition-all duration-300 overflow-hidden',
          sidebarOpen ? 'w-80' : 'w-0'
        )}
      >
        <div className="p-6 space-y-6 w-80">
          <FeaturePanel
            sessionId={sessionId}
            isStreaming={isStreaming}
            messageCount={messages.length}
          />
          <AgentActivity activities={activities} />
        </div>
      </div>
    </div>
  );
}