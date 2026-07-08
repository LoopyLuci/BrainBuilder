import { useEffect, useMemo, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import { MetricPoint } from '../api/metrics';
import { Diagnostic, diagnoseTraining } from '../api/tauri';
import { DiagnosticsList } from '../intent/DiagnosticsList';
import { Panel } from '../ui/Panel';
import { HelpTip } from '../help/HelpTip';

const WIDTH = 400;
const HEIGHT = 140;
const PAD = 10;

function LossChart({ points }: { points: MetricPoint[] }) {
  const path = useMemo(() => {
    if (points.length < 2) return '';
    const losses = points.map((p) => p.loss);
    const min = Math.min(...losses);
    const max = Math.max(...losses);
    const range = max - min || 1;
    const xStep = (WIDTH - PAD * 2) / (points.length - 1);
    return points
      .map((p, i) => {
        const x = PAD + i * xStep;
        const y = HEIGHT - PAD - ((p.loss - min) / range) * (HEIGHT - PAD * 2);
        return `${i === 0 ? 'M' : 'L'}${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(' ');
  }, [points]);

  if (points.length < 2) {
    return (
      <div className="bb-empty" style={{ height: HEIGHT, display: 'flex', alignItems: 'center', justifyContent: 'center' }}>
        Loss curve appears once training starts
      </div>
    );
  }

  return (
    <svg
      viewBox={`0 0 ${WIDTH} ${HEIGHT}`}
      style={{ width: '100%', height: HEIGHT, background: 'var(--bg-subtle)', borderRadius: 'var(--radius-md)', border: '1px solid var(--border-subtle)' }}
    >
      <path d={path} fill="none" style={{ stroke: 'var(--accent)' }} strokeWidth={1.75} strokeLinejoin="round" strokeLinecap="round" />
    </svg>
  );
}

export function TrainingDashboard() {
  const [metrics, setMetrics] = useState<MetricPoint[]>([]);
  const [trainingDiags, setTrainingDiags] = useState<Diagnostic[]>([]);

  useEffect(() => {
    const unlisten = listen<MetricPoint>('metrics-update', (event) => {
      setMetrics((prev) => {
        // A new training run restarts epoch/step at 0 — start a fresh curve
        // instead of appending a discontinuous jump onto the previous run's.
        const isNewRun = prev.length > 0 && event.payload.step === 0 && event.payload.epoch === 0;
        return isNewRun ? [event.payload] : [...prev, event.payload];
      });
    });
    return () => {
      unlisten.then((fn) => fn());
    };
  }, []);

  // Interpret the loss curve in plain English, debounced so rapid metric
  // updates coalesce into one check (each new point resets the timer, so the
  // analysis runs ~0.6s after training pauses or finishes).
  useEffect(() => {
    if (metrics.length < 3) {
      setTrainingDiags([]);
      return;
    }
    const losses = metrics.map((m) => m.loss);
    const timer = setTimeout(() => {
      diagnoseTraining(losses).then(setTrainingDiags).catch(() => {});
    }, 600);
    return () => clearTimeout(timer);
  }, [metrics]);

  const latest = metrics[metrics.length - 1];
  const first = metrics[0];

  return (
    <Panel title="Training Metrics">
      <LossChart points={metrics} />
      {latest && (
        <div>
          <div>
            Epoch <HelpTip term="epoch" /> {latest.epoch}, Step {latest.step}: Loss <HelpTip term="loss" /> ={' '}
            {latest.loss.toFixed(4)}
          </div>
          {first && metrics.length > 1 && (
            <div className={latest.loss < first.loss ? 'bb-text-success' : 'bb-text-error'}>
              {latest.loss < first.loss
                ? `↓ ${(((first.loss - latest.loss) / first.loss) * 100).toFixed(1)}% since start`
                : '↑ loss increasing'}
            </div>
          )}
        </div>
      )}
      {trainingDiags.length > 0 && (
        <div style={{ marginTop: 10 }}>
          <DiagnosticsList diagnostics={trainingDiags} />
        </div>
      )}
    </Panel>
  );
}
