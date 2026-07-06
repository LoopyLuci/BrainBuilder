import { create } from 'zustand';
import { WidgetDef, WidgetSlot } from './types';

// The runtime widget registry, as a zustand store so that registering or
// replacing a widget at runtime (e.g. from a hot-loaded plugin) re-renders the
// shell with no reload. Registration is idempotent by id: registering an
// existing id *replaces* it, which is exactly how a widget gets hot-swapped.

interface RegistryState {
  widgets: Record<string, WidgetDef>;
  register: (def: WidgetDef) => void;
  unregister: (id: string) => void;
  bySlot: (slot: WidgetSlot) => WidgetDef[];
}

export const useWidgetRegistry = create<RegistryState>((set, get) => ({
  widgets: {},
  register: (def) =>
    set((state) => ({ widgets: { ...state.widgets, [def.id]: { order: 100, source: 'builtin', ...def } } })),
  unregister: (id) =>
    set((state) => {
      const next = { ...state.widgets };
      delete next[id];
      return { widgets: next };
    }),
  bySlot: (slot) =>
    Object.values(get().widgets)
      .filter((w) => w.slot === slot)
      .sort((a, b) => (a.order ?? 100) - (b.order ?? 100)),
}));

// Convenience for non-React callers (plugin host, builtins bootstrap).
export function registerWidget(def: WidgetDef) {
  useWidgetRegistry.getState().register(def);
}
