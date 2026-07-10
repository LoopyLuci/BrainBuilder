import { useEffect, useState } from 'react';
import { save } from '@tauri-apps/api/dialog';
import { useGraphStore } from '../state/graphStore';
import { convertToBBIR } from '../canvas/utils';
import {
  predict,
  batchPredict,
  hasCheckpoint,
  exportCheckpoint,
  featureImportance,
  listCheckpointVersions,
  restoreCheckpointVersion,
  startPredictServer,
  stopPredictServer,
  predictServerStatus,
  PredictResult,
  FeatureImportance,
  CheckpointVersion,
} from '../api/tauri';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { HelpTip } from '../help/HelpTip';
import { confirmAction } from '../ui/confirmStore';

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
  const [batchRunning, setBatchRunning] = useState(false);
  const [batchError, setBatchError] = useState<string | null>(null);
  const [batchResult, setBatchResult] = useState<{ rows: number; path: string } | null>(null);
  const [serverUrl, setServerUrl] = useState<string | null>(null);
  const [serverBusy, setServerBusy] = useState(false);
  const [serverError, setServerError] = useState<string | null>(null);
  const [importances, setImportances] = useState<FeatureImportance[] | null>(null);
  const [explaining, setExplaining] = useState(false);
  const [explainError, setExplainError] = useState<string | null>(null);
  // Feature importance only makes sense for tabular data with real named
  // columns — image/text sources don't have a meaningful "column" to rank.
  const isFileSource = training.data_source.source_type === 'file';
  const [versions, setVersions] = useState<CheckpointVersion[] | null>(null);
  const [restoringId, setRestoringId] = useState<string | null>(null);

  const refreshVersions = () => {
    listCheckpointVersions(graphId)
      .then(setVersions)
      .catch(() => {
        /* best-effort — version history is a bonus, not core functionality */
      });
  };

  useEffect(() => {
    let cancelled = false;
    const check = () =>
      hasCheckpoint(graphId)
        .then((exists) => {
          if (cancelled) return;
          setCheckpointExists(exists);
          // Keep polling version history alongside checkpoint existence —
          // retraining an already-trained graph doesn't flip `exists` (it
          // was already true), it archives a new version underneath it, so
          // this can't be gated to just the false→true transition.
          if (exists) refreshVersions();
        })
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
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [graphId]);

  useEffect(() => {
    // Recovers UI state after a reload — the server itself lives in the
    // Rust backend and keeps running across a frontend refresh, so without
    // this the button would falsely offer "Start" while one is already up.
    predictServerStatus()
      .then(setServerUrl)
      .catch(() => {
        /* best-effort — worst case the button just offers to (re)start it */
      });
  }, []);

  const runStartServer = async () => {
    setServerError(null);
    setServerBusy(true);
    try {
      const url = await startPredictServer();
      setServerUrl(url);
      logInfo(`Local predict server started at ${url}.`);
    } catch (e) {
      setServerError(String(e));
      logError(`Starting the predict server failed: ${e}`);
    } finally {
      setServerBusy(false);
    }
  };

  const runStopServer = async () => {
    setServerBusy(true);
    try {
      await stopPredictServer();
      setServerUrl(null);
      logInfo('Local predict server stopped.');
    } catch (e) {
      setServerError(String(e));
      logError(`Stopping the predict server failed: ${e}`);
    } finally {
      setServerBusy(false);
    }
  };

  const runRestore = async (version: CheckpointVersion) => {
    const when = new Date(Number(version.id)).toLocaleString();
    const ok = await confirmAction({
      title: 'Restore this earlier version?',
      body:
        `This replaces the current checkpoint with the one from ${when}. The checkpoint it replaces is ` +
        "archived first, so this is never a one-way door either — you can always restore back.",
      confirmLabel: 'Yes, restore it',
    });
    if (!ok) return;
    setRestoringId(version.id);
    try {
      await restoreCheckpointVersion(graphId, version.id);
      logInfo(`Restored the checkpoint from ${when}.`);
      refreshVersions();
    } catch (e) {
      logError(`Restoring that version failed: ${e}`);
    } finally {
      setRestoringId(null);
    }
  };

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

  const runExplain = async () => {
    setExplainError(null);
    setImportances(null);
    setExplaining(true);
    try {
      const graph = convertToBBIR(nodes, edges, graphId, 'untitled', training);
      const ranked = await featureImportance(graph, training.data_source.path_or_uri, 20);
      setImportances(ranked);
    } catch (e) {
      setExplainError(String(e));
      logError(`Explaining the model failed: ${e}`);
    } finally {
      setExplaining(false);
    }
  };

  const runBatchPredict = async () => {
    let destPath: string | null;
    try {
      destPath = await save({ filters: [{ name: 'CSV predictions', extensions: ['csv'] }] });
    } catch (e) {
      logError(`Couldn't open the export dialog: ${e}`);
      return;
    }
    if (!destPath) return;
    setBatchError(null);
    setBatchResult(null);
    setBatchRunning(true);
    try {
      const graph = convertToBBIR(nodes, edges, graphId, 'untitled', training);
      const rows = await batchPredict(graph, training.data_source.path_or_uri, destPath);
      setBatchResult({ rows, path: destPath });
      logInfo(`Batch prediction wrote ${rows} row(s) to ${destPath}.`);
    } catch (e) {
      setBatchError(String(e));
      logError(`Batch prediction failed: ${e}`);
    } finally {
      setBatchRunning(false);
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
      {checkpointExists && (
        <div className="bb-row" style={{ alignItems: 'center', gap: 6, marginTop: 4 }}>
          <Button
            variant="secondary"
            data-tutorial="batch-predict-btn"
            onClick={runBatchPredict}
            disabled={batchRunning || !training.data_source.path_or_uri}
          >
            {batchRunning ? 'Running…' : 'Run on entire dataset & export CSV…'}
          </Button>
          <span className="bb-text-muted" style={{ fontSize: 12 }}>
            Every row, not just a preview handful — a real <HelpTip term="batch-inference" /> pass, written straight
            to a CSV file.
          </span>
        </div>
      )}
      {batchError && <div className="bb-text-error">{batchError}</div>}
      {batchResult && (
        <p className="bb-text-muted" data-tutorial="batch-predict-result" style={{ margin: '4px 0 0' }}>
          Wrote {batchResult.rows} prediction{batchResult.rows === 1 ? '' : 's'} to {batchResult.path}.
        </p>
      )}
      {checkpointExists && (
        <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 8 }}>
          <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
            Let another program ask this model for predictions live, over the network <HelpTip term="serving" />
            {' '}— no export needed.
          </p>
          <div className="bb-row" style={{ alignItems: 'center', gap: 6 }}>
            {!serverUrl ? (
              <Button variant="secondary" data-tutorial="serve-model-btn" onClick={runStartServer} disabled={serverBusy}>
                {serverBusy ? 'Starting…' : 'Start local server'}
              </Button>
            ) : (
              <Button variant="secondary" data-tutorial="serve-model-btn" onClick={runStopServer} disabled={serverBusy}>
                {serverBusy ? 'Stopping…' : 'Stop server'}
              </Button>
            )}
          </div>
          {serverError && <div className="bb-text-error" style={{ marginTop: 4 }}>{serverError}</div>}
          {serverUrl && (
            <div data-tutorial="serve-model-url" style={{ marginTop: 6, fontFamily: 'var(--font-mono)', fontSize: 11 }}>
              <div>{serverUrl}/predict</div>
              <div className="bb-text-muted" style={{ marginTop: 4, whiteSpace: 'pre-wrap' }}>
                {`curl -X POST ${serverUrl}/predict \\\n  -H "Content-Type: application/json" \\\n  -d '{"graph_json": "<your graph JSON>", "dataset_path": "your_data.csv", "rows": 5}'`}
              </div>
            </div>
          )}
        </div>
      )}
      {checkpointExists && (
        <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 8 }}>
          <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
            Every time you train over an existing checkpoint, the one it replaces is saved here automatically —
            training again is never a one-way door, and neither is a rollback <HelpTip term="rollback" />.
          </p>
          {versions && versions.length === 0 && (
            <p className="bb-text-muted" data-tutorial="versions-empty">
              No earlier versions yet — train this model again and the checkpoint it replaces will show up here.
            </p>
          )}
          {versions && versions.length > 0 && (
            <ul className="bb-list" data-tutorial="versions-list">
              {versions.map((v) => (
                <li
                  key={v.id}
                  className="bb-list-item"
                  style={{ fontSize: 11, fontFamily: 'var(--font-mono)', display: 'flex', alignItems: 'center', gap: 6 }}
                >
                  <span style={{ flex: 1 }}>{new Date(Number(v.id)).toLocaleString()}</span>
                  <span className="bb-text-muted">{(v.size_bytes / 1024).toFixed(1)} KB</span>
                  <Button
                    variant="ghost"
                    data-tutorial="restore-version-btn"
                    onClick={() => runRestore(v)}
                    disabled={restoringId !== null}
                  >
                    {restoringId === v.id ? 'Restoring…' : 'Restore'}
                  </Button>
                </li>
              ))}
            </ul>
          )}
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
      {checkpointExists && isFileSource && (
        <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 8 }}>
          <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
            Which columns is the model actually paying attention to? <HelpTip term="feature-importance" />
          </p>
          <Button variant="secondary" data-tutorial="explain-btn" onClick={runExplain} disabled={explaining}>
            {explaining ? 'Explaining…' : 'Explain this model…'}
          </Button>
          {explainError && <div className="bb-text-error" style={{ marginTop: 4 }}>{explainError}</div>}
          {importances && importances.length === 0 && (
            <p className="bb-text-muted" style={{ marginTop: 6 }}>
              Only one column feeds this model, so there's nothing to compare it against.
            </p>
          )}
          {importances && importances.length > 0 && (
            <ul className="bb-list" data-tutorial="explain-results" style={{ marginTop: 6 }}>
              {importances.map((imp, i) => (
                <li
                  key={imp.column}
                  className="bb-list-item"
                  style={{ fontSize: 11, fontFamily: 'var(--font-mono)', display: 'flex', alignItems: 'center', gap: 6 }}
                >
                  <span className="bb-text-muted" style={{ width: 18 }}>#{i + 1}</span>
                  <span style={{ flex: 1 }}>{imp.column}</span>
                  <span>{imp.importance.toFixed(4)}</span>
                </li>
              ))}
            </ul>
          )}
        </div>
      )}
    </Panel>
  );
}
