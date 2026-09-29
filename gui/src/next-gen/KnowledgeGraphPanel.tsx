import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface KgEntity {
  id: string;
  name: string;
  kind: string;
}

export function KnowledgeGraphPanel() {
  const [entityId, setEntityId] = useState('');
  const [entityName, setEntityName] = useState('');
  const [entityKind, setEntityKind] = useState('concept');
  const [neighbors, setNeighbors] = useState<Array<{ source: string; target: string; relation_type: string; weight: number }>>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const addEntity = async () => {
    if (!entityId.trim() || !entityName.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke('kg_add_entity', {
        entity_json: JSON.stringify({ id: entityId.trim(), name: entityName.trim(), kind: entityKind }),
      });
      setEntityId('');
      setEntityName('');
      logInfo(`kg_add_entity: ${entityId}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`kg_add_entity failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const loadNeighbors = async () => {
    if (!entityId.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const n = await invoke<Array<{ source: string; target: string; relation_type: string; weight: number }>>('kg_neighbors', {
        node_id: entityId.trim(),
      });
      setNeighbors(n);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`kg_neighbors failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Knowledge Graph" subtitle="Entity and relation graph exploration.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Entity</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={entityId}
            onChange={(e) => setEntityId(e.target.value)}
            placeholder="ID"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            value={entityName}
            onChange={(e) => setEntityName(e.target.value)}
            placeholder="Name"
            style={{ flex: 1 }}
          />
          <select
            className="bb-input"
            value={entityKind}
            onChange={(e) => setEntityKind(e.target.value)}
            style={{ flex: 1 }}
          >
            <option value="concept">Concept</option>
            <option value="entity">Entity</option>
            <option value="event">Event</option>
          </select>
        </div>
        <div className="bb-row" style={{ marginTop: 6 }}>
          <Button variant="primary" onClick={addEntity} disabled={busy || !entityId.trim() || !entityName.trim()}>
            Add Entity
          </Button>
          <Button variant="ghost" onClick={loadNeighbors} disabled={busy || !entityId.trim()}>
            Neighbors
          </Button>
        </div>
      </div>

      {neighbors.length > 0 && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>Neighbors</div>
          <div style={{ maxHeight: 160, overflow: 'auto' }}>
            <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Source</th>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Target</th>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Relation</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Weight</th>
                </tr>
              </thead>
              <tbody>
                {neighbors.map((n, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                    <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{n.source}</td>
                    <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{n.target}</td>
                    <td style={{ padding: '3px 4px' }}>{n.relation_type}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{n.weight.toFixed(2)}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </Panel>
  );
}
