import { useEffect, useState } from 'react';
import { save } from '@tauri-apps/api/dialog';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR } from '../canvas/utils';
import { predict, hasCheckpoint, exportCheckpoint, PredictResult } from '../api/tauri';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { HelpTip } from '../help/HelpTip';

export function PredictPanel() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const training = useGraphStore((s) => s.training);
  const [checkpointExists, setCheckpointExists] = useState(false);
  const [results, setResults] = useState<PredictResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [exporting, setExporting] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const check = () =>
      hasCheckpoint(graphId)
        .then((exists) => !cancelled && setCheckpointExists(exists))
        .catch(() => {
          /* best-effort polling — a transient failure just retries next tick */
        });
    check();
    // No "training complete" event exists yet — poll until a checkpoint
    // shows up (training saves one at the end of the run) so the button
    // enables itself without the user needing to switch tabs and back.
    const interval = setInterval(() => {
      if (!cancelled) check();
    }, 3000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, [graphId]);

  const runPredict = async () => {
    setError(null);
    setResults(null);
    setBusy(true);
    try {
      const graph = convertToBBIR(nodes, edges, graphId, 'untitled', training);
      const rows = await predict(graph, training.data_source.path_or_uri, 5);
      setResults(rows);
    } catch (e) {
      setError(String(e));
      logError(`Predict failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const runExport = async () => {
    let destPath: string | null;
    try {
      destPath = await save({ filters: [{ name: 'PyTorch checkpoint', extensions: ['pt'] }] });
    } catch (e) {
      logError(`Couldn't open the export dialog: ${e}`);
      return;
    }
    if (!destPath) return;
    setExporting(true);
    try {
      await exportCheckpoint(graphId, destPath);
      logInfo(`Checkpoint exported to ${destPath}.`);
    } catch (e) {
      logError(`Exporting the checkpoint failed: ${e}`);
    } finally {
      setExporting(false);
    }
  };

  return (
    <Panel title="Predict">
      {!checkpointExists && (
        <p className="bb-text-muted" data-tutorial="predict-empty">
          No trained checkpoint yet for this graph — train it first.
        </p>
      )}
      <Button
        variant="primary"
        data-tutorial="predict-run-btn"
        onClick={runPredict}
        disabled={!checkpointExists || busy || !training.data_source.path_or_uri}
      >
        {busy ? 'Running…' : 'Run on first 5 rows'}
      </Button>
      {checkpointExists && (
        <div className="bb-row" style={{ alignItems: 'center', gap: 6, marginTop: 4 }}>
          <Button variant="secondary" data-tutorial="export-checkpoint-btn" onClick={runExport} disabled={exporting}>
            {exporting ? 'Exporting…' : 'Export checkpoint…'}
          </Button>
          <span className="bb-text-muted" style={{ fontSize: 12 }}>
            Copies a standard PyTorch checkpoint file, ready for deployment <HelpTip term="deployment" /> outside
            BrainBuilder.
          </span>
        </div>
      )}
      {error && <div className="bb-text-error">{error}</div>}
      {results && (
        <div style={{ fontFamily: 'var(--font-mono)', fontSize: 11 }}>
          {results.map((r, i) => (
            <div key={i}>
              shape [{r.shape.join(', ')}]: {r.values.map((v) => v.toFixed(4)).join(', ')}
            </div>
          ))}
        </div>
      )}
    </Panel>
  );
}
