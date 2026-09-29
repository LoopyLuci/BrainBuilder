import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface HpoResultView {
  best_score: number;
  params: Record<string, number>;
}

export function HyperparameterOptimizerPanel() {
  const [paramsJson, setParamsJson] = useState('{"learning_rate":0.01,"dropout":0.1}');
  const [score, setScore] = useState('0.5');
  const [result, setResult] = useState<HpoResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const step = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<HpoResultView>('hpo_step', { params_json: paramsJson, score: Number(score), elapsed_ms: 100 });
      setResult(r);
      logInfo(`hpo_step: best=${r.best_score.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`hpo_step failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const best = async () => {
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const b = await invoke<HpoResultView>('hpo_best');
      setResult(b);
      logInfo(`hpo_best: ${b.best_score.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`hpo_best failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Hyperparameter Optimizer" subtitle="Random-search HPO loop.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Params JSON</div>
        <textarea
          className="bb-input"
          value={paramsJson}
          onChange={(e) => setParamsJson(e.target.value)}
          style={{ width: '100%', minHeight: 60, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={score}
            onChange={(e) => setScore(e.target.value)}
            placeholder="Score"
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={step} disabled={busy}>
            Step
          </Button>
          <Button variant="ghost" onClick={best} disabled={busy}>
            Best
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ fontSize: 12 }}>
            <span className="bb-text-muted">Best score: </span>
            <span style={{ fontWeight: 600 }}>{result.best_score.toFixed(2)}</span>
          </div>
          <div style={{ fontSize: 11, color: 'rgb(148,163,184)', fontFamily: 'var(--font-mono)' }}>
            {JSON.stringify(result.params)}
          </div>
        </div>
      )}
    </Panel>
  );
}
