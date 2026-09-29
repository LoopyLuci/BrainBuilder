import { useEffect, useState } from 'react';
import { ExperimentRecord, listExperiments } from '../api/tauri';
import { logError } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { HelpTip } from '../help/HelpTip';

const LIMIT = 25;

// Every finished training run is logged automatically (see execute_graph in
// main.rs) — this panel just reads that real history back, newest first, so
// past results are there to compare instead of needing to be remembered.
export function ExperimentsPanel() {
  const [runs, setRuns] = useState<ExperimentRecord[] | null>(null);
  const [loading, setLoading] = useState(false);

  const refresh = () => {
    setLoading(true);
    listExperiments(LIMIT)
      .then(setRuns)
      .catch((e) => logError(`Loading experiment history failed: ${e}`))
      .finally(() => setLoading(false));
  };

  useEffect(() => {
    refresh();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return (
    <Panel
      title="Experiment History"
      action={
        <Button variant="secondary" data-tutorial="experiments-refresh-btn" onClick={refresh} disabled={loading}>
          {loading ? 'Loading…' : 'Refresh'}
        </Button>
      }
    >
      <p className="bb-text-muted" style={{ margin: 0 }}>
        Every time you train, BrainBuilder quietly logs the run <HelpTip term="experiment" /> here — so you can
        compare settings later instead of needing to remember what you tried.
      </p>
      {runs && runs.length === 0 && (
        <p className="bb-empty" data-tutorial="experiments-empty">
          No training runs yet — train a model and it'll show up here.
        </p>
      )}
      {runs && runs.length > 0 && (
        <div style={{ overflow: 'auto', maxHeight: 320 }} data-tutorial="experiments-table">
          <table className="bb-table">
            <thead>
              <tr>
                <th>Model</th>
                <th>
                  Architecture <HelpTip term="architecture" />
                </th>
                <th>When</th>
                <th>Loss fn</th>
                <th>Optimizer</th>
                <th>LR</th>
                <th>Epochs</th>
                <th>Loss: first → last</th>
              </tr>
            </thead>
            <tbody>
              {runs.map((r, i) => {
                const hasBoth = r.first_loss !== null && r.last_loss !== null;
                const improved = hasBoth && (r.last_loss as number) < (r.first_loss as number);
                const pct =
                  hasBoth && r.first_loss !== 0
                    ? (((r.first_loss as number) - (r.last_loss as number)) / Math.abs(r.first_loss as number)) * 100
                    : null;
                return (
                  <tr key={i}>
                    <td>{r.graph_name}</td>
                    <td data-tutorial="experiments-architecture">{r.architecture || '—'}</td>
                    <td>{new Date(r.logged_at).toLocaleString()}</td>
                    <td>{r.loss_fn}</td>
                    <td>{r.optimizer}</td>
                    <td>{r.lr}</td>
                    <td>{r.epochs}</td>
                    <td className={hasBoth ? (improved ? 'bb-text-success' : 'bb-text-error') : undefined}>
                      {hasBoth
                        ? `${(r.first_loss as number).toFixed(4)} → ${(r.last_loss as number).toFixed(4)}${
                            pct !== null ? ` (${pct >= 0 ? '↓' : '↑'}${Math.abs(pct).toFixed(0)}%)` : ''
                          }`
                        : '—'}
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}
    </Panel>
  );
}
