import { CheckCircle2, Archive, MessageCircle, Layers, Zap } from 'lucide-react';
import { cn } from '../lib/utils';

interface FeaturePanelProps {
  sessionId: string | null;
  isStreaming: boolean;
  messageCount: number;
}

export function FeaturePanel({ sessionId, messageCount }: FeaturePanelProps) {
  const features = [
    {
      icon: Archive,
      label: 'Checkpointing',
      description: 'Session persistence',
      active: !!sessionId,
    },
    {
      icon: MessageCircle,
      label: 'Summarization',
      description: 'Context optimized',
      active: messageCount > 5,
    },
    {
      icon: Layers,
      label: 'Sub-Agents',
      description: '6 specialized AI agents',
      active: true,
    },
  ];

  return (
    <div className="rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-lg overflow-hidden">
      <div className="px-5 py-4 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-blue-50 to-purple-50 dark:from-blue-950/30 dark:to-purple-950/30">
        <div className="flex items-center gap-2">
          <Zap className="w-5 h-5 text-blue-600 dark:text-blue-400" />
          <h3 className="font-bold text-slate-900 dark:text-white">Active Features</h3>
        </div>
      </div>

      <div className="p-5 space-y-4">
        {features.map((feature, index) => {
          const Icon = feature.icon;
          return (
            <div key={index} className="flex items-start gap-3 group">
              <div
                className={cn(
                  'flex-shrink-0 p-2.5 rounded-xl transition-all duration-200',
                  feature.active
                    ? 'bg-gradient-to-br from-emerald-500 to-teal-600 shadow-lg shadow-emerald-500/20'
                    : 'bg-slate-100 dark:bg-slate-700'
                )}
              >
                <Icon
                  className={cn(
                    'w-4 h-4 transition-colors',
                    feature.active ? 'text-white' : 'text-slate-400 dark:text-slate-500'
                  )}
                />
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2 mb-1">
                  <span className="text-sm font-semibold text-slate-900 dark:text-white">
                    {feature.label}
                  </span>
                  {feature.active && (
                    <CheckCircle2 className="w-3.5 h-3.5 text-emerald-500 animate-pulse" />
                  )}
                </div>
                <p className="text-xs text-slate-500 dark:text-slate-400">
                  {feature.description}
                </p>
              </div>
            </div>
          );
        })}
      </div>
    </div>
  );
}