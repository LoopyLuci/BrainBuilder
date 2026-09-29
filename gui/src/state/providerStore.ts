import { create } from 'zustand';
import { listLlmProviders, listProviderModels } from '../api/models';

// The currently-selected LLM provider + model, shared across every feature
// that calls an LLM (Author, component synthesis, the agent). Kept tiny and
// separate from graphStore since it's app-wide config, not document state.
// The resolved selector ("provider:model") is what the backend commands take.

const LAST_SELECTOR_KEY = 'brainbuilder.llm.lastSelector';

interface ProviderState {
  providers: [string, string][]; // [id, display_name]
  models: string[]; // models for the selected provider
  provider: string;
  model: string;
  loading: boolean;
  error: string | null;
  /** Fetch the provider list + the selected provider's models. */
  refresh: () => Promise<void>;
  setProvider: (id: string) => Promise<void>;
  setModel: (model: string) => void;
  /** The "provider:model" string the backend commands expect. */
  selector: () => string;
}

function persist(provider: string, model: string) {
  try {
    localStorage.setItem(LAST_SELECTOR_KEY, `${provider}:${model}`);
  } catch {
    /* localStorage unavailable — non-fatal */
  }
}

function restore(): { provider: string; model: string } {
  try {
    const raw = localStorage.getItem(LAST_SELECTOR_KEY);
    if (raw) {
      const idx = raw.indexOf(':');
      if (idx > 0) return { provider: raw.slice(0, idx), model: raw.slice(idx + 1) };
    }
  } catch {
    /* ignore */
  }
  return { provider: 'ollama', model: 'llama3.2' };
}

export const useProviderStore = create<ProviderState>((set, get) => ({
  providers: [],
  models: [],
  ...restore(),
  loading: false,
  error: null,
  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const providers = await listLlmProviders();
      const current = get().provider;
      // Fall back to the first available provider if the persisted one is gone.
      const provider = providers.some(([id]) => id === current) ? current : providers[0]?.[0] ?? 'ollama';
      const models = await listProviderModels(provider);
      const model = models.includes(get().model) ? get().model : models[0] ?? get().model;
      set({ providers, provider, models, model, loading: false });
      persist(provider, model);
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
  setProvider: async (id) => {
    set({ provider: id, loading: true, error: null });
    try {
      const models = await listProviderModels(id);
      const model = models[0] ?? get().model;
      set({ models, model, loading: false });
      persist(id, model);
    } catch (e) {
      set({ error: String(e), loading: false });
    }
  },
  setModel: (model) => {
    set({ model });
    persist(get().provider, model);
  },
  selector: () => `${get().provider}:${get().model}`,
}));
