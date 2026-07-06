import { useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { previewDataset, DatasetPreview } from '../api/tauri';
import { useGraphStore } from '../state/graphStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

const LOSSES = ['mse', 'cross_entropy'];
const OPTIMIZERS = ['sgd', 'adam'];

export function DataPanel() {
  const training = useGraphStore((s) => s.training);
  const setTraining = useGraphStore((s) => s.setTraining);
  const [preview, setPreview] = useState<DatasetPreview | null>(null);
  const [error, setError] = useState<string | null>(null);

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
            Must match the <code className="bb-code">vocab_size</code> hyperparameter on your graph's{' '}
            <code className="bb-code">embedding</code> node — not auto-synced yet.
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
    </Panel>
  );
}
