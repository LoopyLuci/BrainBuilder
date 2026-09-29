import { useEffect } from 'react';
import { useProviderStore } from '../state/providerStore';

// Reusable provider + model picker, backed by the shared providerStore. Any
// feature that drives an LLM (Author, component synthesis, the agent) drops
// this in and reads `useProviderStore.getState().selector()` when it calls.
export function ProviderSelector() {
  const { providers, models, provider, model, loading, error, refresh, setProvider, setModel } =
    useProviderStore();

  // Populate the provider/model lists on first mount (idempotent — refresh is
  // cheap and re-entrant).
  useEffect(() => {
    if (providers.length === 0) void refresh();
  }, [providers.length, refresh]);

  return (
    <div className="bb-form-grid">
      <label className="bb-label">Provider</label>
      <select
        className="bb-select"
        value={provider}
        onChange={(e) => void setProvider(e.target.value)}
        disabled={loading}
      >
        {providers.map(([id, name]) => (
          <option key={id} value={id}>
            {name}
          </option>
        ))}
      </select>

      <label className="bb-label">Model</label>
      {models.length > 0 ? (
        <select className="bb-select" value={model} onChange={(e) => setModel(e.target.value)} disabled={loading}>
          {models.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      ) : (
        // Ollama with nothing pulled (or an unreachable provider) — let the
        // user still type a model name rather than being stuck with no options.
        <input
          className="bb-input"
          value={model}
          onChange={(e) => setModel(e.target.value)}
          placeholder="model name"
        />
      )}

      {error && (
        <p className="bb-text-error" style={{ gridColumn: '1 / -1', margin: 0 }}>
          {error}
        </p>
      )}
    </div>
  );
}
