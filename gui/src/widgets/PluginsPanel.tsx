import { useState } from 'react';
import { usePluginLoader } from './loader';
import { WidgetManifest } from './types';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

// Lists loaded runtime plugins and lets the user load a new one from a manifest
// URL/path. Loading a plugin registers its widgets live; unloading removes them
// — no rebuild, no reload. This is the user-facing control surface for the
// full runtime plugin-loading capability.
export function PluginsPanel() {
  const { plugins, load, unload } = usePluginLoader();
  const [entry, setEntry] = useState('');

  const loadFromEntry = async () => {
    const trimmed = entry.trim();
    if (!trimmed) return;
    // A bare entry URL is enough; id/version default from the URL until the
    // module supplies richer metadata. Real bundles ship a full manifest.
    const manifest: WidgetManifest = {
      id: trimmed.split('/').pop()?.replace(/\.[jt]sx?$/, '') ?? trimmed,
      version: '0.0.0',
      entry: trimmed,
      capabilities: ['register-widget', 'read-graph'],
    };
    await load(manifest);
    const state = usePluginLoader.getState().plugins[manifest.id];
    if (state?.status === 'error') logError(`Plugin "${manifest.id}" failed to load: ${state.error}`);
    else logInfo(`Plugin "${manifest.id}" loaded.`);
    setEntry('');
  };

  const entries = Object.values(plugins);

  return (
    <Panel title="Plugins" subtitle="Load UI widgets at runtime — capability-gated, no rebuild.">
      <div className="bb-row">
        <input
          className="bb-input"
          value={entry}
          onChange={(e) => setEntry(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && loadFromEntry()}
          placeholder="plugin module URL (ES module exporting register)"
        />
        <Button variant="secondary" onClick={loadFromEntry} disabled={!entry.trim()}>
          Load
        </Button>
      </div>

      {entries.length === 0 && <div className="bb-empty">No plugins loaded.</div>}

      <ul className="bb-list">
        {entries.map(({ manifest, status, error }) => (
          <li key={manifest.id} className="bb-list-item" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <div style={{ minWidth: 0 }}>
              <div style={{ fontWeight: 600 }}>{manifest.id}</div>
              <div className="bb-text-muted" style={{ fontSize: 11 }}>
                {status === 'error' ? `error: ${error}` : `v${manifest.version} · ${(manifest.capabilities ?? []).join(', ')}`}
              </div>
            </div>
            <Button variant="ghost" onClick={() => unload(manifest.id)}>
              Unload
            </Button>
          </li>
        ))}
      </ul>
    </Panel>
  );
}
