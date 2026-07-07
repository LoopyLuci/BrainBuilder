import { create } from 'zustand';

// Persisted, user-controlled overrides on top of the widget registry. The
// registry says which widgets *exist*; the layout store says which the user has
// chosen to *hide*, and it survives reloads. Kept separate from the registry so
// that hot-(un)loading a plugin (registry churn) never clobbers layout intent,
// and vice-versa.

const HIDDEN_KEY = 'brainbuilder.layout.hidden';

function restore(): Set<string> {
  try {
    const raw = localStorage.getItem(HIDDEN_KEY);
    if (raw) return new Set<string>(JSON.parse(raw));
  } catch {
    /* ignore */
  }
  return new Set();
}

function persist(hidden: Set<string>) {
  try {
    localStorage.setItem(HIDDEN_KEY, JSON.stringify([...hidden]));
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

interface LayoutState {
  hidden: Set<string>;
  isHidden: (id: string) => boolean;
  toggle: (id: string) => void;
  show: (id: string) => void;
}

export const useLayoutStore = create<LayoutState>((set, get) => ({
  hidden: restore(),
  isHidden: (id) => get().hidden.has(id),
  toggle: (id) =>
    set((s) => {
      const hidden = new Set(s.hidden);
      if (hidden.has(id)) hidden.delete(id);
      else hidden.add(id);
      persist(hidden);
      return { hidden };
    }),
  show: (id) =>
    set((s) => {
      if (!s.hidden.has(id)) return s;
      const hidden = new Set(s.hidden);
      hidden.delete(id);
      persist(hidden);
      return { hidden };
    }),
}));
