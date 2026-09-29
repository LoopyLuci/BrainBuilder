import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface ContinualProgressView {
  accuracy: number;
  loss: number;
  forgetting: number;
}

export function ContinualLearningPanel() {
  const [accuracy, setAccuracy] = useState('0.9');
  const [loss, setLoss] = useState('0.1');
  const [progress, setProgress] = useState<{ accuracy: number; loss: number } | null>(null);
  const [forgetting, setForgetting] = useState<{ accuracy: number; loss: number; forgetting: number } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const train = async () => {
    setBusy(true);
    setError(null);
    try {
      const r = await invoke<ContinualProgressView>('continual_train', {
        samples: 1,
        accuracy: Number(accuracy),
        loss: Number(loss),
      });
      setProgress({ accuracy: r.accuracy, loss: r.loss });
      logInfo(`continual_train: acc=${r.accuracy.toFixed(2)} loss=${r.loss.toFixed(2)}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`continual_train failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const checkForgetting = async () => {
    setBusy(true);
    setError(null);
    try {
      const f = await invoke<ContinualProgressView>('continual_forgetting');
      setForgetting({ accuracy: f.accuracy, loss: f.loss, forgetting: f.forgetting });
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`continual_forgetting failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Continual Learning" subtitle="Task progression and forgetting metrics.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Train step</div>
        <div className="bb-row">
          <input
            className="bb-input"
            type="number"
            value={accuracy}
            onChange={(e) => setAccuracy(e.target.value)}
            placeholder="Accuracy"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            type="number"
            value={loss}
            onChange={(e) => setLoss(e.target.value)}
            placeholder="Loss"
            style={{ flex: 1 }}
          />
          <Button variant="primary" onClick={train} disabled={busy}>
            Train
          </Button>
        </div>
      </div>

      <div className="bb-row">
        <Button variant="ghost" onClick={checkForgetting} disabled={busy}>
          Check Forgetting
        </Button>
      </div>

      {progress && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Accuracy: </span>
              <span style={{ fontWeight: 600 }}>{progress.accuracy.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Loss: </span>
              <span style={{ fontWeight: 600 }}>{progress.loss.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
      {forgetting && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap' }}>
            <div>
              <span className="bb-text-muted">Accuracy: </span>
              <span style={{ fontWeight: 600 }}>{forgetting.accuracy.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Loss: </span>
              <span style={{ fontWeight: 600 }}>{forgetting.loss.toFixed(2)}</span>
            </div>
            <div>
              <span className="bb-text-muted">Forgetting: </span>
              <span style={{ fontWeight: 600 }}>{forgetting.forgetting.toFixed(2)}</span>
            </div>
          </div>
        </div>
      )}
    </Panel>
  );
}
