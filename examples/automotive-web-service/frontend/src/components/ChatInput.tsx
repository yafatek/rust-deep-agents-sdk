import { useState, KeyboardEvent } from 'react';
import { Send, Sparkles } from 'lucide-react';
import { cn } from '../lib/utils';

interface ChatInputProps {
  onSend: (message: string) => void;
  disabled?: boolean;
}

export function ChatInput({ onSend, disabled = false }: ChatInputProps) {
  const [input, setInput] = useState('');

  const handleSubmit = () => {
    if (input.trim() && !disabled) {
      onSend(input.trim());
      setInput('');
    }
  };

  const handleKeyDown = (e: KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === 'Enter' && !e.shiftKey) {
      e.preventDefault();
      handleSubmit();
    }
  };

  return (
    <div className="px-6 py-4 bg-white/80 dark:bg-slate-900/80 backdrop-blur-xl border-t border-slate-200 dark:border-slate-800">
      <div className="max-w-4xl mx-auto">
        <div className="relative flex items-end gap-3">
          <div className="flex-1 relative">
            <textarea
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={handleKeyDown}
              placeholder="Ask about diagnostics, bookings, or anything else..."
              disabled={disabled}
              rows={1}
              className={cn(
                'w-full px-5 py-4 pr-12 rounded-2xl resize-none',
                'bg-white dark:bg-slate-800',
                'border-2 border-slate-200 dark:border-slate-700',
                'focus:border-blue-500 dark:focus:border-blue-400',
                'focus:outline-none focus:ring-4 focus:ring-blue-500/10',
                'text-slate-900 dark:text-white',
                'placeholder:text-slate-400 dark:placeholder:text-slate-500',
                'transition-all duration-200',
                'disabled:opacity-50 disabled:cursor-not-allowed',
                'shadow-sm hover:shadow-md',
                'min-h-[56px] max-h-[200px]'
              )}
              style={{
                scrollbarWidth: 'thin',
                scrollbarColor: 'rgb(148 163 184) transparent'
              }}
            />

            {!disabled && input.trim() && (
              <div className="absolute right-3 bottom-3">
                <Sparkles className="w-5 h-5 text-blue-500 animate-pulse" />
              </div>
            )}
          </div>

          <button
            onClick={handleSubmit}
            disabled={disabled || !input.trim()}
            className={cn(
              'flex-shrink-0 w-12 h-12 rounded-2xl',
              'bg-gradient-to-r from-blue-600 to-purple-600',
              'hover:from-blue-700 hover:to-purple-700',
              'shadow-lg shadow-blue-500/25',
              'transition-all duration-200',
              'disabled:opacity-50 disabled:cursor-not-allowed disabled:shadow-none',
              'flex items-center justify-center',
              'hover:scale-105 active:scale-95'
            )}
          >
            <Send className="w-5 h-5 text-white" />
          </button>
        </div>

        <p className="mt-3 text-xs text-slate-400 dark:text-slate-500 text-center">
          Press Enter to send • Shift + Enter for new line
        </p>
      </div>
    </div>
  );
}