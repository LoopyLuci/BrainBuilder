import { ComponentType } from 'react';

// Where a widget mounts in the app shell. The shell renders each slot by
// querying the registry, so adding/removing/reordering panels never touches
// App.tsx — the precondition for hot-swapping UI at runtime.
export type WidgetSlot = 'palette' | 'side' | 'bottom' | 'canvas' | 'header';

// One registered piece of UI. `component` is a normal React component; the
// shell wraps every widget in an error boundary so a broken one (including a
// freshly hot-loaded plugin) is contained, never fatal to the shell.
export interface WidgetDef {
  id: string;
  title: string;
  slot: WidgetSlot;
  component: ComponentType;
  /** Lower renders first within a slot. Defaults to 100. */
  order?: number;
  /** Marks a widget that came from a runtime plugin (vs. a built-in). */
  source?: 'builtin' | 'plugin';
}

// A plugin bundle's self-description, validated before its module is loaded.
// `entry` is a URL/path to an ES module exposing `register(host)`.
export interface WidgetManifest {
  id: string;
  version: string;
  entry: string;
  /** Human-facing description shown in the Plugins panel. */
  description?: string;
  /** Coarse capabilities the plugin declares it needs (gated by the host). */
  capabilities?: PluginCapability[];
}

// The narrow, typed surface a runtime plugin is handed — deliberately NOT the
// raw Tauri `invoke`. A plugin can register widgets and call a curated set of
// host actions gated by its declared capabilities, mirroring the backend's
// deny-by-default capability model.
export type PluginCapability = 'register-widget' | 'read-graph' | 'author-llm';

export interface PluginHost {
  registerWidget: (def: Omit<WidgetDef, 'source'>) => void;
  /** Only present if the plugin declared the matching capability. */
  readGraph?: () => { nodes: unknown[]; edges: unknown[] };
}
