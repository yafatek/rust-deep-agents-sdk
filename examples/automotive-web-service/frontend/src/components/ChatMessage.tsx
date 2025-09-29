import { cn } from '../lib/utils';
import type { Message } from '../types';
import { Bot, User } from 'lucide-react';

interface ChatMessageProps {
  message: Message;
  isStreaming?: boolean;
}

export function ChatMessage({ message, isStreaming = false }: ChatMessageProps) {
  const isUser = message.role === 'user';

  return (
    <div className={cn(
      'flex gap-4 group',
      isUser && 'flex-row-reverse'
    )}>
      {/* Avatar */}
      <div className={cn(
        'flex-shrink-0 w-10 h-10 rounded-xl flex items-center justify-center shadow-md',
        isUser
          ? 'bg-gradient-to-br from-emerald-500 to-teal-600'
          : 'bg-gradient-to-br from-blue-500 to-purple-600'
      )}>
        {isUser ? (
          <User className="w-5 h-5 text-white" />
        ) : (
          <Bot className="w-5 h-5 text-white" />
        )}
      </div>

      {/* Message Content */}
      <div className={cn(
        'flex-1 min-w-0',
        isUser && 'flex flex-col items-end'
      )}>
        <div className="flex items-center gap-2 mb-2">
          <span className={cn(
            'text-sm font-semibold',
            isUser ? 'text-emerald-600 dark:text-emerald-400' : 'text-blue-600 dark:text-blue-400'
          )}>
            {isUser ? 'You' : 'AI Assistant'}
          </span>
          <span className="text-xs text-slate-400 dark:text-slate-500">
            {message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
          </span>
        </div>

        <div className={cn(
          'px-5 py-3 rounded-2xl shadow-sm',
          isUser
            ? 'bg-gradient-to-br from-emerald-50 to-teal-50 dark:from-emerald-950/30 dark:to-teal-950/30 border border-emerald-200 dark:border-emerald-900/50'
            : 'bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700'
        )}>
          <p className="text-[15px] leading-relaxed text-slate-900 dark:text-slate-100 whitespace-pre-wrap">
            {message.content}
            {isStreaming && (
              <span className="inline-flex items-center ml-1">
                <span className="w-1.5 h-4 bg-blue-600 dark:bg-blue-400 animate-pulse rounded-full"></span>
              </span>
            )}
          </p>
        </div>
      </div>
    </div>
  );
}