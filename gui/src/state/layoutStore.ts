import { create } from 'zustand';

// Persisted, user-controlled overrides on top of the widget registry. The
// registry says which widgets *exist*; the layout store says which the user has
// chosen to *hide*, and it survives reloads. Kept separate from the registry so
// that hot-(un)loading a plugin (registry churn) never clobbers layout intent,
// and vice-versa.

const HIDDEN_KEY = 'brainbuilder.layout.hidden';
const SIDE_RAIL_KEY = 'brainbuilder.layout.sideRailVisible';
const RIGHT_RAIL_KEY = 'brainbuilder.layout.rightRailVisible';
const PALETTE_KEY = 'brainbuilder.layout.paletteVisible';
const BOTTOM_HEIGHT_KEY = 'brainbuilder.layout.bottomHeight';
const SIDE_WIDTH_KEY = 'brainbuilder.layout.sideWidth';
const RIGHT_WIDTH_KEY = 'brainbuilder.layout.rightWidth';

function restoreHidden(): Set<string> {
  try {
    const raw = localStorage.getItem(HIDDEN_KEY);
    if (raw) return new Set<string>(JSON.parse(raw));
  } catch {
    /* ignore */
  }
  return new Set();
}

function persistHidden(hidden: Set<string>) {
  try {
    localStorage.setItem(HIDDEN_KEY, JSON.stringify([...hidden]));
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

function restoreBoolean(key: string, fallback: boolean): boolean {
  try {
    const raw = localStorage.getItem(key);
    if (raw === '1') return true;
    if (raw === '0') return false;
  } catch {
    /* ignore */
  }
  return fallback;
}

function persistBoolean(key: string, value: boolean) {
  try {
    localStorage.setItem(key, value ? '1' : '0');
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

function restoreNumber(key: string, fallback: number): number {
  try {
    const raw = localStorage.getItem(key);
    const n = Number(raw);
    if (Number.isFinite(n)) return n;
  } catch {
    /* ignore */
  }
  return fallback;
}

function persistNumber(key: string, value: number) {
  try {
    localStorage.setItem(key, String(value));
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

interface LayoutState {
  hidden: Set<string>;
  isHidden: (id: string) => boolean;
  toggle: (id: string) => void;
  show: (id: string) => void;
  sideRailVisible: boolean;
  toggleSideRail: () => void;
  setSideRailVisible: (value: boolean) => void;
  rightRailVisible: boolean;
  toggleRightRail: () => void;
  setRightRailVisible: (value: boolean) => void;
  paletteVisible: boolean;
  togglePalette: () => void;
  setPaletteVisible: (value: boolean) => void;
  bottomHeight: number;
  setBottomHeight: (value: number) => void;
  sideWidth: number;
  setSideWidth: (value: number) => void;
  rightWidth: number;
  setRightWidth: (value: number) => void;
}

export const useLayoutStore = create<LayoutState>((set, get) => ({
  hidden: restoreHidden(),
  isHidden: (id) => get().hidden.has(id),
  toggle: (id) =>
    set((s) => {
      const hidden = new Set(s.hidden);
      if (hidden.has(id)) hidden.delete(id);
      else hidden.add(id);
      persistHidden(hidden);
      return { hidden };
    }),
  show: (id) =>
    set((s) => {
      if (!s.hidden.has(id)) return s;
      const hidden = new Set(s.hidden);
      hidden.delete(id);
      persistHidden(hidden);
      return { hidden };
    }),
  sideRailVisible: restoreBoolean(SIDE_RAIL_KEY, true),
  toggleSideRail: () =>
    set((s) => {
      const next = !s.sideRailVisible;
      persistBoolean(SIDE_RAIL_KEY, next);
      return { sideRailVisible: next };
    }),
  setSideRailVisible: (value) =>
    set((s) => {
      if (s.sideRailVisible === value) return s;
      persistBoolean(SIDE_RAIL_KEY, value);
      return { sideRailVisible: value };
    }),
  rightRailVisible: restoreBoolean(RIGHT_RAIL_KEY, true),
  toggleRightRail: () =>
    set((s) => {
      const next = !s.rightRailVisible;
      persistBoolean(RIGHT_RAIL_KEY, next);
      return { rightRailVisible: next };
    }),
  setRightRailVisible: (value) =>
    set((s) => {
      if (s.rightRailVisible === value) return s;
      persistBoolean(RIGHT_RAIL_KEY, value);
      return { rightRailVisible: value };
    }),
  paletteVisible: restoreBoolean(PALETTE_KEY, true),
  togglePalette: () =>
    set((s) => {
      const next = !s.paletteVisible;
      persistBoolean(PALETTE_KEY, next);
      return { paletteVisible: next };
    }),
  setPaletteVisible: (value) =>
    set((s) => {
      if (s.paletteVisible === value) return s;
      persistBoolean(PALETTE_KEY, value);
      return { paletteVisible: value };
    }),
  bottomHeight: restoreNumber(BOTTOM_HEIGHT_KEY, 260),
  setBottomHeight: (value) =>
    set((s) => {
      const clamped = Math.min(520, Math.max(120, value));
      if (s.bottomHeight === clamped) return s;
      persistNumber(BOTTOM_HEIGHT_KEY, clamped);
      return { bottomHeight: clamped };
    }),
  sideWidth: restoreNumber(SIDE_WIDTH_KEY, 340),
  setSideWidth: (value) =>
    set((s) => {
      const clamped = Math.min(520, Math.max(220, value));
      if (s.sideWidth === clamped) return s;
      persistNumber(SIDE_WIDTH_KEY, clamped);
      return { sideWidth: clamped };
    }),
  rightWidth: restoreNumber(RIGHT_WIDTH_KEY, 280),
  setRightWidth: (value) =>
    set((s) => {
      const clamped = Math.min(520, Math.max(220, value));
      if (s.rightWidth === clamped) return s;
      persistNumber(RIGHT_WIDTH_KEY, clamped);
      return { rightWidth: clamped };
    }),
}));
