import { create } from 'zustand';

export interface LogEntry {
  type: 'info' | 'warn' | 'error' | 'system';
  text: string;
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
    set((state) => ({
      logs: [...state.logs, { type, text: formattedText }].slice(-500),
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
