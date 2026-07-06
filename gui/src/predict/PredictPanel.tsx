import { useEffect, useState } from 'react';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR } from '../canvas/utils';
import { predict, hasCheckpoint, PredictResult } from '../api/tauri';
import { logError } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

export function PredictPanel() {
  const graphId = useGraphStore((s) => s.graphId);
  const nodes = useGraphStore((s) => s.nodes);
  const edges = useGraphStore((s) => s.edges);
  const training = useGraphStore((s) => s.training);
  const [checkpointExists, setCheckpointExists] = useState(false);
  const [results, setResults] = useState<PredictResult[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    let cancelled = false;
    const check = () => hasCheckpoint(graphId).then((exists) => !cancelled && setCheckpointExists(exists));
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

  return (
    <Panel title="Predict">
      {!checkpointExists && <p className="bb-text-muted">No trained checkpoint yet for this graph — train it first.</p>}
      <Button variant="primary" onClick={runPredict} disabled={!checkpointExists || busy || !training.data_source.path_or_uri}>
        {busy ? 'Running…' : 'Run on first 5 rows'}
      </Button>
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
