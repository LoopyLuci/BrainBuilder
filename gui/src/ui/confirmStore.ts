import { create } from 'zustand';

// A promise-based confirmation gate for destructive/irreversible actions
// (anything that can't be fully undone with Ctrl+Z — e.g. wiping the canvas,
// replacing it with a loaded file). One dialog at a time, rendered by
// <ConfirmDialog/> in App.tsx; call `confirm(...)` from anywhere and `await`
// the user's choice instead of acting immediately.
interface ConfirmRequest {
  title: string;
  body: string;
  confirmLabel: string;
  danger: boolean;
  resolve: (ok: boolean) => void;
}

interface ConfirmState {
  request: ConfirmRequest | null;
  ask: (opts: { title: string; body: string; confirmLabel?: string; danger?: boolean }) => Promise<boolean>;
  resolve: (ok: boolean) => void;
}

export const useConfirmStore = create<ConfirmState>((set, get) => ({
  request: null,
  ask: (opts) =>
    new Promise<boolean>((resolve) => {
      set({
        request: {
          title: opts.title,
          body: opts.body,
          confirmLabel: opts.confirmLabel ?? 'Yes, continue',
          danger: opts.danger ?? true,
          resolve,
        },
      });
    }),
  resolve: (ok) => {
    const req = get().request;
    if (!req) return;
    set({ request: null });
    req.resolve(ok);
  },
}));

/** Convenience wrapper: `if (await confirmAction({...})) { ...do the thing... }` */
export function confirmAction(opts: { title: string; body: string; confirmLabel?: string; danger?: boolean }) {
  return useConfirmStore.getState().ask(opts);
}
