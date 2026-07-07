import { create } from 'zustand';
import * as React from 'react';
import { registerWidget, useWidgetRegistry } from './registry';
import { PluginHost, WidgetManifest, PluginCapability } from './types';
import { useGraphStore } from '../state/graphStore';
import { useProviderStore } from '../state/providerStore';
import { generateGraph } from '../api/models';

// Runtime plugin loading. A plugin is an ES module exposing `register(host)`;
// we validate its manifest, hand it a NARROW capability-gated host API (never
// raw `invoke`), and let it register widgets that appear live in the shell.
// This is the UI-layer analogue of the backend nervous system: untrusted code
// gets only the capabilities it declared, nothing more.

const LOADED_KEY = 'brainbuilder.plugins.enabled';

interface LoadedPlugin {
  manifest: WidgetManifest;
  status: 'loaded' | 'error';
  error?: string;
}

interface PluginState {
  plugins: Record<string, LoadedPlugin>;
  load: (manifest: WidgetManifest) => Promise<void>;
  unload: (id: string) => void;
}

// Build the host surface a given plugin is allowed to touch, based on the
// capabilities it declared. A capability it didn't ask for simply isn't there.
function buildHost(manifest: WidgetManifest): PluginHost {
  const caps = new Set<PluginCapability>(manifest.capabilities ?? []);
  const host: PluginHost = {
    react: React,
    registerWidget: (def) => {
      if (!caps.has('register-widget')) {
        throw new Error(`plugin "${manifest.id}" tried to register a widget without the 'register-widget' capability`);
      }
      // Namespace the id so a plugin can't clobber a built-in widget by id.
      registerWidget({ ...def, id: `plugin:${manifest.id}:${def.id}`, source: 'plugin' });
    },
  };
  if (caps.has('read-graph')) {
    host.readGraph = () => {
      const { nodes, edges } = useGraphStore.getState();
      return { nodes, edges };
    };
  }
  if (caps.has('author-llm')) {
    // Gated LLM access: the plugin can request a graph, but the provider,
    // model, and API key stay app-side (never handed to the plugin).
    host.authorGraph = (description: string) => generateGraph(description, useProviderStore.getState().selector());
  }
  return host;
}

// Import a plugin module from a URL/path. We fetch the source as text and
// instantiate it via a Blob URL rather than importing the path directly: this
// bypasses the bundler entirely (the Vite dev server otherwise rewrites
// dynamic imports of public-dir JS with a `?import` suffix that 404s), works
// identically in a production Tauri build, and gives us the source text as a
// natural place to inspect/validate untrusted plugin code before running it.
async function importPluginModule(entry: string): Promise<{ register?: unknown }> {
  const res = await fetch(entry);
  if (!res.ok) throw new Error(`couldn't fetch plugin at ${entry} (HTTP ${res.status})`);
  const code = await res.text();
  const url = URL.createObjectURL(new Blob([code], { type: 'text/javascript' }));
  try {
    return await import(/* @vite-ignore */ url);
  } finally {
    URL.revokeObjectURL(url);
  }
}

// Persist the *manifests* of currently-loaded plugins (not just their ids) so
// they can be re-loaded verbatim on the next launch. Only successfully-loaded
// plugins are stored, so a plugin that errored won't be re-attempted forever.
function persistLoaded(plugins: Record<string, LoadedPlugin>) {
  try {
    const manifests = Object.values(plugins)
      .filter((p) => p.status === 'loaded')
      .map((p) => p.manifest);
    localStorage.setItem(LOADED_KEY, JSON.stringify(manifests));
  } catch {
    /* ignore */
  }
}

// Re-load every plugin that was active at last shutdown. Called once at
// startup (App bootstrap) after the built-in widgets register, so a user's
// hot-loaded panels survive a restart without a rebuild.
export async function restorePlugins(): Promise<void> {
  let manifests: WidgetManifest[] = [];
  try {
    const raw = localStorage.getItem(LOADED_KEY);
    if (raw) manifests = JSON.parse(raw);
  } catch {
    return;
  }
  const { load } = usePluginLoader.getState();
  for (const manifest of manifests) {
    // Ignore anything that isn't a well-formed manifest (older format, etc.).
    if (manifest && typeof manifest.id === 'string' && typeof manifest.entry === 'string') {
      await load(manifest);
    }
  }
}

export const usePluginLoader = create<PluginState>((set, get) => ({
  plugins: {},
  load: async (manifest) => {
    // Minimal manifest validation before we ever import the code.
    if (!manifest.id || !manifest.entry) {
      set((s) => ({
        plugins: { ...s.plugins, [manifest.id || '?']: { manifest, status: 'error', error: 'manifest missing id/entry' } },
      }));
      return;
    }
    try {
      const mod = await importPluginModule(manifest.entry);
      if (typeof mod.register !== 'function') {
        throw new Error("plugin module has no exported `register(host)` function");
      }
      mod.register(buildHost(manifest));
      set((s) => ({ plugins: { ...s.plugins, [manifest.id]: { manifest, status: 'loaded' } } }));
      persistLoaded(get().plugins);
    } catch (e) {
      set((s) => ({
        plugins: { ...s.plugins, [manifest.id]: { manifest, status: 'error', error: String(e) } },
      }));
    }
  },
  unload: (id) => {
    // Remove the plugin's widgets, then forget the plugin.
    const registry = useWidgetRegistry.getState();
    Object.keys(registry.widgets)
      .filter((wid) => wid.startsWith(`plugin:${id}:`))
      .forEach((wid) => registry.unregister(wid));
    set((s) => {
      const next = { ...s.plugins };
      delete next[id];
      persistLoaded(next);
      return { plugins: next };
    });
  },
}));
