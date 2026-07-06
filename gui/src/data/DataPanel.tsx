import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { previewDataset, DatasetPreview } from '../api/tauri';
import { autotune, TrialResult } from '../api/models';
import { convertToBBIR } from '../canvas/utils';
import { useGraphStore } from '../state/graphStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

const LOSSES = ['mse', 'cross_entropy'];
const OPTIMIZERS = ['sgd', 'adam'];

export function DataPanel() {
  const training = useGraphStore((s) => s.training);
  const setTraining = useGraphStore((s) => s.setTraining);
  const nodes = useGraphStore((s) => s.nodes);
  const updateNodeHyperparams = useGraphStore((s) => s.updateNodeHyperparams);
  const [preview, setPreview] = useState<DatasetPreview | null>(null);
  const [error, setError] = useState<string | null>(null);

  // Keep every embedding node's `vocab_size` hyperparameter in lockstep with
  // the dataset's vocab_size so a text-sequence run can't silently mismatch
  // the embedding table shape against the tokenizer.
  const vocabSize = training.data_source.vocab_size;
  useEffect(() => {
    if (vocabSize === undefined) return;
    for (const node of nodes) {
      if (node.data.component !== 'embedding') continue;
      if (node.data.hyperparams?.vocab_size === vocabSize) continue;
      updateNodeHyperparams(node.id, { ...node.data.hyperparams, vocab_size: vocabSize });
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [vocabSize, nodes]);

  const pickDataset = async () => {
    const selected = await open({
      multiple: false,
      filters: [{ name: 'Dataset', extensions: ['csv', 'parquet', 'txt'] }],
    });
    if (typeof selected !== 'string') return;
    setError(null);
    // .txt is the one source_type that isn't tabular — real word-level
    // tokenization + sliding-window sequence loading (core/src/data/text.rs)
    // for training embedding/attention components, instead of the
    // csv/parquet path's (batch, features) tabular preview.
    const isText = selected.toLowerCase().endsWith('.txt');
    setTraining({
      ...training,
      data_source: {
        ...training.data_source,
        source_type: isText ? 'text_sequence' : 'file',
        path_or_uri: selected,
        sequence_length: isText ? training.data_source.sequence_length ?? 32 : undefined,
        vocab_size: isText ? training.data_source.vocab_size ?? 5000 : undefined,
      },
    });
    if (isText) {
      setPreview(null);
      return;
    }
    try {
      setPreview(await previewDataset(selected, 8));
    } catch (e) {
      setError(String(e));
      setPreview(null);
    }
  };

  const edges = useGraphStore((s) => s.edges);
  const graphId = useGraphStore((s) => s.graphId);
  const [tuning, setTuning] = useState(false);
  const [trials, setTrials] = useState<TrialResult[] | null>(null);

  const runAutotune = async () => {
    setTuning(true);
    setTrials(null);
    try {
      const graphJson = JSON.stringify(convertToBBIR(nodes, edges, graphId, 'untitled', training));
      const ranked = await autotune(graphJson, 8);
      setTrials(ranked);
      const best = ranked.find((t) => t.score !== null);
      if (best) {
        // Apply the winning config so the user can just train with it.
        setTraining({
          ...training,
          optimizer: best.optimizer,
          hyperparams: { ...training.hyperparams, lr: best.learning_rate ?? training.hyperparams?.lr },
          data_source: { ...training.data_source, batch_size: best.batch_size },
        });
        logInfo(`Auto-tune picked lr=${best.learning_rate}, ${best.optimizer}, batch=${best.batch_size} (loss ${best.score?.toFixed(4)}).`);
      } else {
        logError('Auto-tune finished but no trial produced a usable result.');
      }
    } catch (e) {
      logError(`Auto-tune failed: ${e}`);
    } finally {
      setTuning(false);
    }
  };

  const isTextSequence = training.data_source.source_type === 'text_sequence';

  const lr = (training.hyperparams?.lr as number) ?? 0.01;
  const epochs = (training.hyperparams?.epochs as number) ?? 10;

  return (
    <Panel title="Data & Training">
      <Button variant="secondary" onClick={pickDataset}>
        Choose dataset…
      </Button>
      <div className="bb-text-muted" style={{ wordBreak: 'break-all' }}>
        {training.data_source.path_or_uri || 'No dataset selected'}
      </div>
      <p className="bb-text-muted" style={{ margin: 0 }}>
        {isTextSequence ? (
          'Text file: tokenized word-by-word and windowed into (context, next-word) training examples.'
        ) : (
          <>
            Convention: the <strong>last column</strong> is the training target.
          </>
        )}
      </p>

      {isTextSequence && (
        <div className="bb-form-grid">
          <label className="bb-label">Sequence length</label>
          <input
            className="bb-input"
            type="number"
            value={training.data_source.sequence_length ?? 32}
            onChange={(e) =>
              setTraining({ ...training, data_source: { ...training.data_source, sequence_length: Number(e.target.value) } })
            }
          />
          <label className="bb-label">Vocab size</label>
          <input
            className="bb-input"
            type="number"
            value={training.data_source.vocab_size ?? 5000}
            onChange={(e) =>
              setTraining({ ...training, data_source: { ...training.data_source, vocab_size: Number(e.target.value) } })
            }
          />
          <p className="bb-text-muted" style={{ gridColumn: '1 / -1', margin: 0 }}>
            Automatically kept in sync with the <code className="bb-code">vocab_size</code> hyperparameter
            on any <code className="bb-code">embedding</code> node in your graph.
          </p>
        </div>
      )}

      {error && <div className="bb-text-error">{error}</div>}
      {preview && (
        <div style={{ overflow: 'auto', maxHeight: 140 }}>
          <table className="bb-table">
            <thead>
              <tr>
                {preview.columns.map((c) => (
                  <th key={c}>{c}</th>
                ))}
              </tr>
            </thead>
            <tbody>
              {preview.rows.map((row, i) => (
                <tr key={i}>
                  {row.map((cell, j) => (
                    <td key={j}>{cell}</td>
                  ))}
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div className="bb-form-grid">
        <label className="bb-label">Loss</label>
        <select className="bb-select" value={training.loss} onChange={(e) => setTraining({ ...training, loss: e.target.value })}>
          {LOSSES.map((l) => (
            <option key={l} value={l}>
              {l}
            </option>
          ))}
        </select>

        <label className="bb-label">Optimizer</label>
        <select
          className="bb-select"
          value={training.optimizer}
          onChange={(e) => setTraining({ ...training, optimizer: e.target.value })}
        >
          {OPTIMIZERS.map((o) => (
            <option key={o} value={o}>
              {o}
            </option>
          ))}
        </select>

        <label className="bb-label">Learning rate</label>
        <input
          className="bb-input"
          type="number"
          step="0.001"
          value={lr}
          onChange={(e) => setTraining({ ...training, hyperparams: { ...training.hyperparams, lr: Number(e.target.value) } })}
        />

        <label className="bb-label">Epochs</label>
        <input
          className="bb-input"
          type="number"
          value={epochs}
          onChange={(e) => setTraining({ ...training, hyperparams: { ...training.hyperparams, epochs: Number(e.target.value) } })}
        />

        <label className="bb-label">Batch size</label>
        <input
          className="bb-input"
          type="number"
          value={training.data_source.batch_size}
          onChange={(e) =>
            setTraining({ ...training, data_source: { ...training.data_source, batch_size: Number(e.target.value) } })
          }
        />
      </div>

      <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 4 }}>
        <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
          Not sure what to pick? Auto-tune runs a few short trials and applies the config that trains best.
        </p>
        <Button variant="secondary" onClick={runAutotune} disabled={tuning || nodes.length === 0}>
          {tuning ? 'Tuning…' : 'Auto-tune'}
        </Button>
        {trials && (
          <ul className="bb-list" style={{ marginTop: 6 }}>
            {trials.slice(0, 4).map((t) => (
              <li key={t.index} className="bb-list-item bb-text-muted" style={{ fontSize: 11, fontFamily: 'var(--font-mono)' }}>
                lr={t.learning_rate} {t.optimizer} batch={t.batch_size} → {t.score === null ? 'failed' : `loss ${t.score.toFixed(4)}`}
              </li>
            ))}
          </ul>
        )}
      </div>
    </Panel>
  );
}
