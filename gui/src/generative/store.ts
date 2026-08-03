import { create } from "zustand";
import type { GenSurface, GenNode } from "./types";

interface GenState {
  surfaces: GenSurface[];
  formValues: Record<string, string>;
  push: (title: string, root: GenNode) => string;
  remove: (id: string) => void;
  clear: () => void;
  setField: (name: string, value: string) => void;
  /** Replace an existing surface atomically (hot UI swap). */
  replace: (id: string, root: GenNode, title?: string) => void;
}

export const useGenUI = create<GenState>((set) => ({
  surfaces: [],
  formValues: {},
  push: (title, root) => {
    const id = `gen-${Date.now()}-${Math.random().toString(36).slice(2, 7)}`;
    set((s) => ({
      surfaces: [{ id, title, createdAt: Date.now(), root }, ...s.surfaces].slice(0, 24),
    }));
    return id;
  },
  remove: (id) => set((s) => ({ surfaces: s.surfaces.filter((x) => x.id !== id) })),
  clear: () => set({ surfaces: [] }),
  setField: (name, value) =>
    set((s) => ({ formValues: { ...s.formValues, [name]: value } })),
  replace: (id, root, title) =>
    set((s) => ({
      surfaces: s.surfaces.map((x) =>
        x.id === id ? { ...x, root, title: title ?? x.title, createdAt: Date.now() } : x
      ),
    })),
}));
