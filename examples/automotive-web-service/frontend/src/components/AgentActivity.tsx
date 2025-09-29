import { Wrench, Calendar, Ticket, CreditCard, Bell, MessageSquare, Bot, Activity } from 'lucide-react';
import { cn } from '../lib/utils';
import type { AgentActivityItem } from '../types';

interface AgentActivityProps {
  activities: AgentActivityItem[];
}

const agentIcons: Record<string, any> = {
  diagnostic: Wrench,
  booking: Calendar,
  ticketing: Ticket,
  payment: CreditCard,
  notification: Bell,
  feedback: MessageSquare,
  coordinator: Bot,
};

export function AgentActivity({ activities }: AgentActivityProps) {
  if (activities.length === 0) {
    return (
      <div className="rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-lg overflow-hidden">
        <div className="px-5 py-4 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-purple-50 to-pink-50 dark:from-purple-950/30 dark:to-pink-950/30">
          <div className="flex items-center gap-2">
            <Activity className="w-5 h-5 text-purple-600 dark:text-purple-400" />
            <h3 className="font-bold text-slate-900 dark:text-white">Agent Activity</h3>
          </div>
        </div>

        <div className="p-8 text-center">
          <Bot className="w-12 h-12 text-slate-300 dark:text-slate-600 mx-auto mb-3" />
          <p className="text-sm text-slate-500 dark:text-slate-400">
            No agent activity yet
          </p>
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-2xl bg-white dark:bg-slate-800 border border-slate-200 dark:border-slate-700 shadow-lg overflow-hidden">
      <div className="px-5 py-4 border-b border-slate-200 dark:border-slate-700 bg-gradient-to-r from-purple-50 to-pink-50 dark:from-purple-950/30 dark:to-pink-950/30">
        <div className="flex items-center gap-2">
          <Activity className="w-5 h-5 text-purple-600 dark:text-purple-400 animate-pulse" />
          <h3 className="font-bold text-slate-900 dark:text-white">Agent Activity</h3>
          <span className="ml-auto text-xs font-medium px-2 py-1 rounded-full bg-purple-100 dark:bg-purple-900/30 text-purple-600 dark:text-purple-400">
            {activities.length}
          </span>
        </div>
      </div>

      <div className="p-4 space-y-2 max-h-80 overflow-y-auto" style={{
        scrollbarWidth: 'thin',
        scrollbarColor: 'rgb(148 163 184) transparent'
      }}>
        {activities.slice(-10).reverse().map((activity, index) => {
          const Icon = agentIcons[activity.agent.toLowerCase()] || Bot;
          return (
            <div
              key={index}
              className={cn(
                'flex items-center gap-3 p-3 rounded-xl transition-all duration-200',
                activity.status === 'active' && 'bg-blue-50 dark:bg-blue-950/20 border border-blue-200 dark:border-blue-900/50',
                activity.status === 'completed' && 'bg-emerald-50 dark:bg-emerald-950/20 border border-emerald-200 dark:border-emerald-900/50',
                activity.status === 'error' && 'bg-red-50 dark:bg-red-950/20 border border-red-200 dark:border-red-900/50'
              )}
            >
              <div className={cn(
                'flex-shrink-0 p-2 rounded-lg',
                activity.status === 'active' && 'bg-blue-100 dark:bg-blue-900/30',
                activity.status === 'completed' && 'bg-emerald-100 dark:bg-emerald-900/30',
                activity.status === 'error' && 'bg-red-100 dark:bg-red-900/30'
              )}>
                <Icon className={cn(
                  'w-4 h-4',
                  activity.status === 'active' && 'text-blue-600 dark:text-blue-400',
                  activity.status === 'completed' && 'text-emerald-600 dark:text-emerald-400',
                  activity.status === 'error' && 'text-red-600 dark:text-red-400'
                )} />
              </div>

              <div className="flex-1 min-w-0">
                <div className="flex items-center gap-2">
                  <span className="text-sm font-semibold text-slate-900 dark:text-white truncate">
                    {activity.agent}
                  </span>
                  {activity.status === 'active' && (
                    <span className="flex-shrink-0 w-2 h-2 rounded-full bg-blue-500 animate-pulse"></span>
                  )}
                </div>
                <p className="text-xs text-slate-500 dark:text-slate-400 truncate">
                  {activity.action}
                </p>
              </div>

              <span className="text-xs text-slate-400 dark:text-slate-500 flex-shrink-0">
                {activity.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
}