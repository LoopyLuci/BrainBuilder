import { create } from 'zustand';

export type LogLevel = 'info' | 'error';

export interface LogEntry {
  id: number;
  level: LogLevel;
  message: string;
  timestamp: number;
}

let nextId = 0;

interface LogState {
  entries: LogEntry[];
  log: (level: LogLevel, message: string) => void;
  clear: () => void;
}

// A real, visible sink for validation/training/predict errors — previously
// these only went to `console.error` (devtools), invisible to anyone not
// watching devtools, which defeats the point of catching shape errors early
// with "helpful messages."
export const useLogStore = create<LogState>((set) => ({
  entries: [],
  log: (level, message) =>
    set((s) => ({
      entries: [...s.entries.slice(-199), { id: nextId++, level, message, timestamp: Date.now() }],
    })),
  clear: () => set({ entries: [] }),
}));

export function logInfo(message: string): void {
  useLogStore.getState().log('info', message);
}

export function logError(message: string): void {
  useLogStore.getState().log('error', message);
}
