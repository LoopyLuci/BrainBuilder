import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface UncertaintyView {
  mean: number;
  std: number;
  samples: number;
}

export function UncertaintyQuantificationPanel() {
  const [prediction, setPrediction] = useState('0.5');
  const [estimate, setEstimate] = useState<UncertaintyView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const addPrediction = async () => {
    if (!prediction.trim()) return;
    setBusy(true);
    setError(null);
    try {
      await invoke('uq_add_prediction', { prediction: Number(prediction) });
      logInfo(`uq_add_prediction: ${prediction}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`uq_add_prediction failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const estimateUncertainty = async () => {
    setBusy(true);
    setError(null);
    setEstimate(null);
    try {
      const u = await invoke<UncertaintyView>('uq_estimate');
      setEstimate(u);
      logInfo(`uq_estimate: mean=${u.mean.toFixed(2)} std=${u.std.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`uq_estimate failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Uncertainty Quantification" subtitle="Monte Carlo uncertainty estimates.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Prediction</div>
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={prediction}
            onChange={(e) => setPrediction(e.target.value)}
            style={{ flex: 1 }}
          />
          <Button variant="ghost" onClick={addPrediction} disabled={busy || !prediction.trim()}>
            Add
          </Button>
          <Button variant="primary" onClick={estimateUncertainty} disabled={busy}>
            Estimate
          </Button>
        </div>
      </div>

      {estimate && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Mean: </span>
              <span style={{ fontWeight: 600 }}>{estimate.mean.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Std: </span>
              <span style={{ fontWeight: 600 }}>{estimate.std.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Samples: </span>
              <span style={{ fontWeight: 600 }}>{estimate.samples}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
