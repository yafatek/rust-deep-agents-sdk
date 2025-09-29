export interface Message {
  role: 'user' | 'assistant';
  content: string;
  timestamp: Date;
}

export interface SSEEvent {
  type: 'session' | 'delta' | 'done' | 'error';
  data: any;
}

export interface AgentActivityItem {
  agent: string;
  action: string;
  timestamp: Date;
  status: 'active' | 'completed' | 'error';
}