import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface PmiResultView {
  pmi: number;
  count: number;
}

export function PmiAnalyzerPanel() {
  const [a, setA] = useState('');
  const [b, setB] = useState('');
  const [result, setResult] = useState<PmiResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const score = async () => {
    if (!a.trim() || !b.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<PmiResultView>('pmi_score', { a: a.trim(), b: b.trim() });
      setResult(r);
      logInfo(`pmi_score: ${r.pmi.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`pmi_score failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const observe = async () => {
    if (!a.trim() || !b.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke('pmi_observe', { a: a.trim(), b: b.trim() });
      logInfo(`pmi_observe: ${a} + ${b}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`pmi_observe failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="PMI Analyzer" subtitle="Pointwise mutual information stats.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Token pair</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={a}
            onChange={(e) => setA(e.target.value)}
            placeholder="Token A"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            value={b}
            onChange={(e) => setB(e.target.value)}
            placeholder="Token B"
            style={{ flex: 1 }}
          />
        </div>
        <div className="bb-row" style={{ marginTop: 6 }}>
          <Button variant="primary" onClick={score} disabled={busy || !a.trim() || !b.trim()}>
            Score
          </Button>
          <Button variant="ghost" onClick={observe} disabled={busy || !a.trim() || !b.trim()}>
            Observe
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">PMI: </span>
              <span style={{ fontWeight: 600 }}>{result.pmi.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Count: </span>
              <span style={{ fontWeight: 600 }}>{result.count}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
