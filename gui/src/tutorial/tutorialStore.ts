import { create } from 'zustand';
import { CURRICULUM, Tutorial } from './curriculum';

// Drives the guided-tutorial overlay: which tutorial (if any) is running,
// which step, and which tutorials the user has already finished (persisted,
// so "Beginner" tutorials show a checkmark on the next launch). Kept as its
// own store (not folded into layoutStore) since tutorial progress is content
// state, not layout preference — different lifecycle, different concerns.

const COMPLETED_KEY = 'brainbuilder.tutorial.completed';

function restoreCompleted(): Set<string> {
  try {
    const raw = localStorage.getItem(COMPLETED_KEY);
    if (raw) return new Set<string>(JSON.parse(raw));
  } catch {
    /* ignore — non-fatal, tutorials just won't show as completed */
  }
  return new Set();
}

function persistCompleted(completed: Set<string>) {
  try {
    localStorage.setItem(COMPLETED_KEY, JSON.stringify([...completed]));
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

interface TutorialState {
  activeId: string | null;
  stepIndex: number;
  completed: Set<string>;
  start: (id: string) => void;
  next: () => void;
  prev: () => void;
  exit: () => void;
  activeTutorial: () => Tutorial | null;
}

export const useTutorialStore = create<TutorialState>((set, get) => ({
  activeId: null,
  stepIndex: 0,
  completed: restoreCompleted(),

  start: (id) => {
    if (!CURRICULUM.some((t) => t.id === id)) return;
    set({ activeId: id, stepIndex: 0 });
  },

  next: () =>
    set((s) => {
      const tutorial = CURRICULUM.find((t) => t.id === s.activeId);
      if (!tutorial) return s;
      const nextIndex = s.stepIndex + 1;
      if (nextIndex >= tutorial.steps.length) {
        // Finished the last step — mark complete and close the overlay.
        const completed = new Set(s.completed);
        completed.add(tutorial.id);
        persistCompleted(completed);
        return { activeId: null, stepIndex: 0, completed };
      }
      return { stepIndex: nextIndex };
    }),

  prev: () => set((s) => ({ stepIndex: Math.max(0, s.stepIndex - 1) })),

  exit: () => set({ activeId: null, stepIndex: 0 }),

  activeTutorial: () => CURRICULUM.find((t) => t.id === get().activeId) ?? null,
}));
