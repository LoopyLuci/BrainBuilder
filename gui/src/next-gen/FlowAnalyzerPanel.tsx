import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface FlowResultView {
  throughput: number;
  bottlenecks: string[];
}

export function FlowAnalyzerPanel() {
  const [load, setLoad] = useState('1.0');
  const [result, setResult] = useState<FlowResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const simulate = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<FlowResultView>('flow_simulate', { load: Number(load) });
      setResult(r);
      logInfo(`flow_simulate: throughput=${r.throughput.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`flow_simulate failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Flow Analyzer" subtitle="Data/control flow simulation and bottleneck detection.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Load</div>
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={load}
            onChange={(e) => setLoad(e.target.value)}
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={simulate} disabled={busy}>
            Simulate
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginBottom: 8 }}>
            <div>
              <span className="bb-text-muted">Throughput: </span>
              <span style={{ fontWeight: 600 }}>{result.throughput.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Bottlenecks: </span>
              <span style={{ fontWeight: 600 }}>{result.bottlenecks.length}</span>
            </div>
          </div>
          {result.bottlenecks.length > 0 && (
            <div style={{ fontSize: 12, color: 'rgb(148,163,184)' }}>
              {result.bottlenecks.join(', ')}
            </div>
          )}
        </div>
      )}
    </Panel>
  );
}
