import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface FederatedAggregateView {
  clients: number;
  weight: number;
  updated: boolean;
}

export function FederatedLearningPanel() {
  const [updatesJson, setUpdatesJson] = useState('[{"client_id":"c1","delta":0.01}]');
  const [clientId, setClientId] = useState('client-1');
  const [result, setResult] = useState<FederatedAggregateView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const aggregate = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<FederatedAggregateView>('federated_aggregate', { updates_json: updatesJson });
      setResult(r);
      logInfo(`federated_aggregate: clients=${r.clients}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`federated_aggregate failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const registerClient = async () => {
    if (!clientId.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke('federated_register', { client_id: clientId.trim() });
      logInfo(`federated_register: ${clientId}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`federated_register failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Federated Learning" subtitle="Decentralized model aggregation.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Updates JSON</div>
        <textarea
          className="bb-input"
          value={updatesJson}
          onChange={(e) => setUpdatesJson(e.target.value)}
          style={{ width: '100%', minHeight: 60, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <Button variant="primary" onClick={aggregate} disabled={busy}>
          Aggregate
        </Button>
      </div>

      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Register client</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={clientId}
            onChange={(e) => setClientId(e.target.value)}
            placeholder="Client ID"
            style={{ flex: 1 }}
          />
          <Button variant="ghost" onClick={registerClient} disabled={busy || !clientId.trim()}>
            Register
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Clients: </span>
              <span style={{ fontWeight: 600 }}>{result.clients}</span>
            </div>
            <div>
              <span className="bb-text-muted">Weight: </span>
              <span style={{ fontWeight: 600 }}>{result.weight.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Updated: </span>
              <span className={`bb-chip ${result.updated ? 'bb-chip--success' : 'bb-chip--warn'}`}>{result.updated ? 'Yes' : 'No'}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
