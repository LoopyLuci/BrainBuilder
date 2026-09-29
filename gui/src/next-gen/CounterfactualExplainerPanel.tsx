import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface CounterfactualView {
  feature: string;
  change: number;
  new_prediction: number;
  delta: number;
}

export function CounterfactualExplainerPanel() {
  const [inputJson, setInputJson] = useState('{"age": 30, "income": 50000, "score": 700}');
  const [prediction, setPrediction] = useState('0.8');
  const [target, setTarget] = useState('0.2');
  const [explanations, setExplanations] = useState<CounterfactualView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const explain = async () => {
    setBusy(true);
    setError(null);
    setExplanations([]);
    try {
      const ex = await invoke<CounterfactualView[]>('counterfactual_explain', {
        input_json: inputJson,
        prediction: Number(prediction),
        target: Number(target),
      });
      setExplanations(ex);
      logInfo(`counterfactual_explain: ${ex.length} explanations`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`counterfactual_explain failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Counterfactual Explainer" subtitle="Minimal changes to flip predictions.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Input JSON</div>
        <textarea
          className="bb-input"
          value={inputJson}
          onChange={(e) => setInputJson(e.target.value)}
          style={{ width: '100%', minHeight: 60, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={prediction}
            onChange={(e) => setPrediction(e.target.value)}
            placeholder="Prediction"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            type="number"
            value={target}
            onChange={(e) => setTarget(e.target.value)}
            placeholder="Target"
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={explain} disabled={busy}>
            Explain
          </Button>
        </div>
      </div>

      {explanations.length > 0 && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>Explanations</div>
          <div style={{ maxHeight: 160, overflow: 'auto' }}>
            <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Feature</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Change</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>New prediction</th>
                  <th style={{ textAlign: 'right', padding: '4px 4px' }}>Delta</th>
                </tr>
              </thead>
              <tbody>
                {explanations.map((e, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                    <td style={{ padding: '3px 4px' }}>{e.feature}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{e.change.toFixed(2)}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{e.new_prediction.toFixed(2)}</td>
                    <td style={{ padding: '3px 4px', textAlign: 'right' }}>{e.delta.toFixed(2)}</td>
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
