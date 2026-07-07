import { useState } from 'react';
import { usePluginLoader } from './loader';
import { WidgetManifest } from './types';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

// Bundled demo plugins shipped in `public/plugins/`, loadable in one click to
// show off runtime hot-loading. Each is a plain ES module exporting
// `register(host)`; they declare only the capabilities they actually use.
const BUNDLED_EXAMPLES: WidgetManifest[] = [
  {
    id: 'sticky-note',
    version: '1.0.0',
    entry: '/plugins/sticky-note.js',
    description: 'A scratch-note pad in the side rail (register-widget only).',
    capabilities: ['register-widget'],
  },
  {
    id: 'graph-stats',
    version: '1.0.0',
    entry: '/plugins/graph-stats.js',
    description: 'Live node/edge/component counts (uses the read-graph capability).',
    capabilities: ['register-widget', 'read-graph'],
  },
];

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

  const loadExample = async (manifest: WidgetManifest) => {
    await load(manifest);
    const state = usePluginLoader.getState().plugins[manifest.id];
    if (state?.status === 'error') logError(`Example "${manifest.id}" failed to load: ${state.error}`);
    else logInfo(`Example plugin "${manifest.id}" loaded — check the ${manifest.id === 'graph-stats' ? 'bottom' : 'side'} rail.`);
  };

  const entries = Object.values(plugins);

  return (
    <Panel title="Plugins" subtitle="Load UI widgets at runtime — capability-gated, no rebuild.">
      <div style={{ marginBottom: 8 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Bundled examples</div>
        <ul className="bb-list">
          {BUNDLED_EXAMPLES.map((ex) => {
            const loaded = Boolean(plugins[ex.id]);
            return (
              <li
                key={ex.id}
                className="bb-list-item"
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 6 }}
              >
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontWeight: 600 }}>{ex.id}</div>
                  <div className="bb-text-muted" style={{ fontSize: 11 }}>{ex.description}</div>
                </div>
                <Button
                  variant={loaded ? 'ghost' : 'secondary'}
                  onClick={() => (loaded ? unload(ex.id) : loadExample(ex))}
                >
                  {loaded ? 'Unload' : 'Load'}
                </Button>
              </li>
            );
          })}
        </ul>
      </div>

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
