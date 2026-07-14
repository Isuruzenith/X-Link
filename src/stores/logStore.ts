import { create } from 'zustand';

export interface LogEntry {
  type: 'info' | 'warn' | 'error' | 'system';
  text: string;
  timestamp: string;
}

interface LogState {
  logs: LogEntry[];
  autoScroll: boolean;
  
  // Actions
  pushLog: (type: LogEntry['type'], text: string) => void;
  pushSystemLog: (text: string) => void;
  clearLogs: () => void;
  setAutoScroll: (scroll: boolean) => void;
  copyLogs: () => Promise<void>;
}

export const useLogStore = create<LogState>((set, get) => ({
  logs: [],
  autoScroll: true,

  pushLog: (type, text) => {
    const formattedText = text.trim();
    if (!formattedText) return;
    const now = new Date();
    const pad = (n: number) => n.toString().padStart(2, '0');
    const timestamp = `${pad(now.getHours())}:${pad(now.getMinutes())}:${pad(now.getSeconds())}`;
    
    set((state) => ({
      logs: [...state.logs, { type, text: formattedText, timestamp }].slice(-2000),
    }));
  },

  pushSystemLog: (text) => {
    get().pushLog('system', `[System] ${text}`);
  },

  clearLogs: () => set({ logs: [] }),
  
  setAutoScroll: (scroll) => set({ autoScroll: scroll }),

  copyLogs: async () => {
    const logText = get().logs.map((l) => l.text).join('\n');
    await navigator.clipboard.writeText(logText);
    get().pushSystemLog('Copied all logs to clipboard.');
  }
}));
