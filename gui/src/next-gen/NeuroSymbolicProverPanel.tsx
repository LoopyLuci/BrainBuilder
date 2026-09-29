import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface NeuroSymbolicProofView {
  clauses: number;
  steps: number;
  proved: boolean;
}

export function NeuroSymbolicProverPanel() {
  const [goal, setGoal] = useState('example_goal');
  const [result, setResult] = useState<NeuroSymbolicProofView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const prove = async () => {
    if (!goal.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<NeuroSymbolicProofView>('neurosymbolic_prove', { goal: goal.trim(), max_steps: 5 });
      setResult(r);
      logInfo(`neurosymbolic_prove: proved=${r.proved} steps=${r.steps}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`neurosymbolic_prove failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Neuro-Symbolic Prover" subtitle="Logic proving with neural guidance.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Goal</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={goal}
            onChange={(e) => setGoal(e.target.value)}
            placeholder="Enter proof goal…"
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={prove} disabled={busy || !goal.trim()}>
            Prove
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Clauses: </span>
              <span style={{ fontWeight: 600 }}>{result.clauses}</span>
            </div>
            <div>
              <span className="bb-text-muted">Steps: </span>
              <span style={{ fontWeight: 600 }}>{result.steps}</span>
            </div>
            <div>
              <span className="bb-text-muted">Proved: </span>
              <span className={`bb-chip ${result.proved ? 'bb-chip--success' : 'bb-chip--warn'}`}>{result.proved ? 'Yes' : 'No'}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
