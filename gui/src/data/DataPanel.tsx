import { useEffect, useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import { previewDataset, DatasetPreview } from '../api/tauri';
import { autotune, TrialResult } from '../api/models';
import { convertToBBIR } from '../canvas/utils';
import { useGraphStore } from '../state/graphStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { SchemaForm, FormSchema, FormValues } from '../widgets/SchemaForm';

const LOSSES = ['mse', 'cross_entropy'];
const OPTIMIZERS = ['sgd', 'adam'];
const PREPROC_OPS = ['normalize', 'cast'];
const CAST_DTYPES = ['float32', 'int32', 'int64'];

// The core training controls, expressed as a schema instead of hand-written
// JSX — the same procedural-UI path the Inspector and plugins use. New knobs
// appear by adding a field here (or from a descriptor), not by writing markup.
const TRAINING_SCHEMA: FormSchema = {
  fields: [
    { name: 'loss', label: 'Loss', type: 'select', options: LOSSES },
    { name: 'optimizer', label: 'Optimizer', type: 'select', options: OPTIMIZERS },
    { name: 'lr', label: 'Learning rate', type: 'number', step: 0.001 },
    // 0.0 (the default) means off — sgd updates using only the current
    // step's gradient, same as before this field existed. A value in
    // (0, 1) (0.9 is a common choice elsewhere) carries forward a fraction
    // of the previous step's direction, smoothing out noisy per-batch
    // gradients. Meaningless for adam, which has its own built-in adaptive
    // momentum — silently ignored there.
    { name: 'momentum', label: 'Momentum, sgd only (0 = off)', type: 'number', min: 0, max: 1, step: 0.01 },
    { name: 'epochs', label: 'Epochs', type: 'number' },
    { name: 'batch_size', label: 'Batch size', type: 'number' },
    // 0 means off — every configured epoch runs, same as before this field
    // existed. A positive value stops training once that many epochs in a
    // row fail to improve the loss, instead of always running the full
    // budget whether or not it's still helping.
    { name: 'patience', label: 'Early-stop patience (0 = off)', type: 'number', min: 0 },
    // 0.0 means off. A positive value pulls every weight a little toward
    // zero each step (L2 regularization) — a whole-run setting, distinct
    // from a dropout box on the canvas.
    { name: 'weight_decay', label: 'Weight decay (0 = off)', type: 'number', min: 0, step: 0.0001 },
    // 0.0 means off — gradients pass through unmodified, same as before
    // this field existed. A positive value caps the combined gradient norm
    // at that value on every step, tempering any single unusually large
    // update instead of letting it through unchecked.
    { name: 'grad_clip', label: 'Gradient clipping (0 = off)', type: 'number', min: 0, step: 0.1 },
    // 0 means off — the learning rate stays constant for the whole run,
    // same as before this field existed. A positive value halves the
    // learning rate every time that many epochs complete, so later epochs
    // take smaller, more careful steps than the first ones did.
    { name: 'lr_decay_epochs', label: 'Halve learning rate every __ epochs (0 = off)', type: 'number', min: 0 },
    // 0.0 means off — cross_entropy trains against exact one-hot targets,
    // same as before this field existed. A value in (0, 1) blends each
    // target a little toward a uniform distribution over every class,
    // discouraging overconfident predictions. Meaningless for mse.
    { name: 'label_smoothing', label: 'Label smoothing (0 = off, classification only)', type: 'number', min: 0, max: 1, step: 0.01 },
    // Off means every epoch sees batches in exactly the same order, same as
    // before this field existed. On re-shuffles the batch order every
    // epoch — matters most for a folder-of-images dataset, whose batches
    // are decoded in class order by default.
    { name: 'shuffle', label: 'Shuffle batch order every epoch', type: 'boolean' },
  ],
};
// Hidden-width hyperparameters the architecture search resizes — must mirror
// the backend's autotune::SCALABLE_WIDTH_KEYS so the applied model matches the
// tuned one.
const SCALABLE_WIDTH_KEYS = ['hidden', 'hidden_size', 'units', 'features', 'out_features', 'dim', 'd_model'];

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
        text_column: undefined,
        label_column: undefined,
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
      logError(`Previewing dataset failed: ${e}`);
    }
  };

  // Toggling this switches a plain tabular CSV/Parquet ("file") between the
  // numeric-columns reading (`source_type: "file"`) and the bag-of-words
  // free-text reading (`source_type: "text_column"` — core/src/data/
  // tabular_text.rs), the only route to a text_column dataset for a
  // manually-built graph (e.g. the "Text Sentiment Classifier" template) —
  // the Intent panel's own text_column option builds its own graph
  // automatically and doesn't apply to a graph placed from the Templates
  // panel.
  const isTextColumnSource = training.data_source.source_type === 'text_column';
  const setIsTextColumn = (on: boolean) => {
    setTraining({
      ...training,
      data_source: {
        ...training.data_source,
        source_type: on ? 'text_column' : 'file',
        vocab_size: on ? training.data_source.vocab_size ?? 2000 : undefined,
        text_column: on ? training.data_source.text_column : undefined,
        label_column: on ? training.data_source.label_column : undefined,
      },
    });
  };

  const edges = useGraphStore((s) => s.edges);
  const graphId = useGraphStore((s) => s.graphId);
  const [tuning, setTuning] = useState(false);
  const [trials, setTrials] = useState<TrialResult[] | null>(null);
  const [searchArch, setSearchArch] = useState(false);

  const preprocessing = training.data_source.preprocessing ?? [];
  const [newOp, setNewOp] = useState('normalize');
  const [newColumn, setNewColumn] = useState('');
  const [newDtype, setNewDtype] = useState('float32');

  const addPreprocStep = () => {
    const column = newColumn.trim();
    if (!column) return;
    const params: Record<string, unknown> = { column };
    if (newOp === 'cast') params.dtype = newDtype;
    setTraining({
      ...training,
      data_source: { ...training.data_source, preprocessing: [...preprocessing, { op: newOp, params }] },
    });
    setNewColumn('');
  };

  const removePreprocStep = (index: number) => {
    setTraining({
      ...training,
      data_source: { ...training.data_source, preprocessing: preprocessing.filter((_, i) => i !== index) },
    });
  };

  const runAutotune = async () => {
    setTuning(true);
    setTrials(null);
    try {
      const graphJson = JSON.stringify(convertToBBIR(nodes, edges, graphId, 'untitled', training));
      const ranked = await autotune(graphJson, searchArch ? 16 : 8, searchArch);
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
        // If a resized architecture won, apply that width to the graph too so
        // the model the user trains matches the model that tuned best. Mirrors
        // the backend's curated allow-list (autotune::SCALABLE_WIDTH_KEYS).
        if (best.width_scale !== 1) {
          for (const node of nodes) {
            const hp = node.data.hyperparams ?? {};
            let changed = false;
            const next = { ...hp };
            for (const key of SCALABLE_WIDTH_KEYS) {
              const v = hp[key];
              if (typeof v === 'number' && Number.isInteger(v)) {
                next[key] = Math.max(1, Math.round(v * best.width_scale));
                changed = true;
              }
            }
            if (changed) updateNodeHyperparams(node.id, next);
          }
        }
        const archNote = best.width_scale !== 1 ? `, width ×${best.width_scale}` : '';
        logInfo(`Auto-tune picked lr=${best.learning_rate}, ${best.optimizer}, batch=${best.batch_size}${archNote} (loss ${best.score?.toFixed(4)}).`);
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
  // The normalize/cast preprocessing list only actually runs for tabular
  // "file" sources (see core/src/data/source.rs: image_folder and
  // text_sequence/text_column each bypass etl::apply_steps entirely) — hide
  // it rather than showing controls that would silently do nothing.
  const isFileSource = training.data_source.source_type === 'file';

  const lr = (training.hyperparams?.lr as number) ?? 0.01;
  const momentum = (training.hyperparams?.momentum as number) ?? 0;
  const epochs = (training.hyperparams?.epochs as number) ?? 10;
  const patience = (training.hyperparams?.patience as number) ?? 0;
  const weightDecay = (training.hyperparams?.weight_decay as number) ?? 0;
  const gradClip = (training.hyperparams?.grad_clip as number) ?? 0;
  const lrDecayEpochs = (training.hyperparams?.lr_decay_epochs as number) ?? 0;
  const labelSmoothing = (training.hyperparams?.label_smoothing as number) ?? 0;
  const shuffle = (training.hyperparams?.shuffle as boolean) ?? false;

  const trainingValues: FormValues = {
    loss: training.loss,
    optimizer: training.optimizer,
    lr,
    momentum,
    epochs,
    batch_size: training.data_source.batch_size,
    patience,
    weight_decay: weightDecay,
    grad_clip: gradClip,
    lr_decay_epochs: lrDecayEpochs,
    label_smoothing: labelSmoothing,
    shuffle,
  };
  // Distribute the flat form values back into the nested TrainingConfig
  // (lr/momentum/epochs/patience/weight_decay/grad_clip/lr_decay_epochs/
  // label_smoothing/shuffle live under hyperparams; batch_size under
  // data_source).
  const onTrainingChange = (v: FormValues) => {
    setTraining({
      ...training,
      loss: String(v.loss),
      optimizer: String(v.optimizer),
      hyperparams: {
        ...training.hyperparams,
        lr: Number(v.lr),
        momentum: Number(v.momentum),
        epochs: Number(v.epochs),
        patience: Number(v.patience),
        weight_decay: Number(v.weight_decay),
        grad_clip: Number(v.grad_clip),
        lr_decay_epochs: Number(v.lr_decay_epochs),
        label_smoothing: Number(v.label_smoothing),
        shuffle: Boolean(v.shuffle),
      },
      data_source: { ...training.data_source, batch_size: Number(v.batch_size) },
    });
  };

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
        ) : isTextColumnSource ? (
          'Text column: turned into a bag-of-words feature vector per row; the label column becomes the class to predict.'
        ) : (
          <>
            Convention: the <strong>last column</strong> is the training target.
          </>
        )}
      </p>

      {!isTextSequence && training.data_source.path_or_uri && (
        <div>
          <label className="bb-label" data-tutorial="text-column-toggle" style={{ margin: '0 0 6px', display: 'flex', alignItems: 'center', gap: 6 }}>
            <input type="checkbox" checked={isTextColumnSource} onChange={(e) => setIsTextColumn(e.target.checked)} />
            This is text data (e.g. reviews) — classify it by a text column, not plain numbers
          </label>
          {isTextColumnSource && (
            <div className="bb-form-grid">
              <label className="bb-label">Text column</label>
              <input
                className="bb-input"
                data-tutorial="text-column-input"
                placeholder="e.g. review"
                value={training.data_source.text_column ?? ''}
                onChange={(e) =>
                  setTraining({ ...training, data_source: { ...training.data_source, text_column: e.target.value } })
                }
                list={preview ? 'preproc-columns' : undefined}
              />
              <label className="bb-label">Label column (optional)</label>
              <input
                className="bb-input"
                data-tutorial="label-column-input"
                placeholder="defaults to the last column"
                value={training.data_source.label_column ?? ''}
                onChange={(e) =>
                  setTraining({ ...training, data_source: { ...training.data_source, label_column: e.target.value } })
                }
                list={preview ? 'preproc-columns' : undefined}
              />
              <label className="bb-label">Vocab size</label>
              <input
                className="bb-input"
                type="number"
                value={training.data_source.vocab_size ?? 2000}
                onChange={(e) =>
                  setTraining({ ...training, data_source: { ...training.data_source, vocab_size: Number(e.target.value) } })
                }
              />
              <p className="bb-text-muted" style={{ gridColumn: '1 / -1', margin: 0 }}>
                Caps the bag-of-words vocabulary — the first <code className="bb-code">linear</code> node's{' '}
                <code className="bb-code">in_features</code> must match the real realized width (shown once you
                train; usually smaller than this cap for a small dataset).
              </p>
            </div>
          )}
        </div>
      )}

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

      {preview && (
        <datalist id="preproc-columns">
          {preview.columns.map((c) => (
            <option key={c} value={c} />
          ))}
        </datalist>
      )}

      {isFileSource && (
      <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 4 }}>
        <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
          Preprocessing steps run on your data, in order, every time you train or predict — before it ever
          reaches the model.
        </p>
        {preprocessing.length === 0 && (
          <p className="bb-text-muted" data-tutorial="preprocess-empty" style={{ margin: '0 0 6px' }}>
            No preprocessing steps yet — your data trains exactly as it is in the file.
          </p>
        )}
        {preprocessing.length > 0 && (
          <ul className="bb-list" data-tutorial="preprocess-list" style={{ marginBottom: 6 }}>
            {preprocessing.map((s, i) => (
              <li
                key={i}
                className="bb-list-item"
                style={{
                  fontSize: 11,
                  fontFamily: 'var(--font-mono)',
                  display: 'flex',
                  alignItems: 'center',
                  gap: 6,
                }}
              >
                <span style={{ flex: 1 }}>
                  {s.op}({s.params.column}
                  {s.op === 'cast' ? ` → ${s.params.dtype}` : ''})
                </span>
                <Button variant="ghost" data-tutorial="preprocess-remove-btn" onClick={() => removePreprocStep(i)}>
                  ✕
                </Button>
              </li>
            ))}
          </ul>
        )}
        <div className="bb-row" style={{ alignItems: 'center', gap: 6, flexWrap: 'wrap' }}>
          <select
            className="bb-select"
            data-tutorial="preprocess-op-select"
            value={newOp}
            onChange={(e) => setNewOp(e.target.value)}
          >
            {PREPROC_OPS.map((op) => (
              <option key={op} value={op}>
                {op === 'normalize' ? 'normalize (z-score)' : 'cast (change type)'}
              </option>
            ))}
          </select>
          <input
            className="bb-input"
            data-tutorial="preprocess-column-input"
            placeholder="column name"
            value={newColumn}
            onChange={(e) => setNewColumn(e.target.value)}
            style={{ width: 120 }}
            list={preview ? 'preproc-columns' : undefined}
          />
          {newOp === 'cast' && (
            <select
              className="bb-select"
              data-tutorial="preprocess-dtype-select"
              value={newDtype}
              onChange={(e) => setNewDtype(e.target.value)}
            >
              {CAST_DTYPES.map((dt) => (
                <option key={dt} value={dt}>
                  {dt}
                </option>
              ))}
            </select>
          )}
          <Button
            variant="secondary"
            data-tutorial="preprocess-add-btn"
            onClick={addPreprocStep}
            disabled={!newColumn.trim()}
          >
            Add step
          </Button>
        </div>
      </div>
      )}

      <div data-tutorial="training-hyperparams">
        <SchemaForm schema={TRAINING_SCHEMA} values={trainingValues} onChange={onTrainingChange} />
      </div>

      <div style={{ borderTop: '1px solid var(--border, rgba(0,0,0,0.1))', paddingTop: 8, marginTop: 4 }}>
        <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
          Not sure what to pick? Auto-tune runs a few short trials and applies the config that trains best.
        </p>
        <div className="bb-row" style={{ alignItems: 'center', gap: 8 }}>
          <Button variant="secondary" data-tutorial="autotune-btn" onClick={runAutotune} disabled={tuning || nodes.length === 0}>
            {tuning ? 'Tuning…' : 'Auto-tune'}
          </Button>
          <label
            className="bb-text-muted"
            data-tutorial="autotune-search-arch"
            style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 4 }}
          >
            <input type="checkbox" checked={searchArch} onChange={(e) => setSearchArch(e.target.checked)} disabled={tuning} />
            also try narrower / wider models
          </label>
        </div>
        {trials && (
          <ul className="bb-list" data-tutorial="autotune-results" style={{ marginTop: 6 }}>
            {trials.map((t, i) => {
              // Trials arrive ranked best-first; the first with a usable score
              // is the winner whose config we applied above.
              const isBest = t.score !== null && !trials.slice(0, i).some((p) => p.score !== null);
              return (
                <li
                  key={t.index}
                  className="bb-list-item"
                  style={{
                    fontSize: 11,
                    fontFamily: 'var(--font-mono)',
                    display: 'flex',
                    alignItems: 'center',
                    gap: 6,
                    color: isBest ? 'var(--text)' : 'var(--text-muted, inherit)',
                  }}
                >
                  <span className="bb-text-muted" style={{ width: 18 }}>#{i + 1}</span>
                  <span style={{ flex: 1 }}>
                    lr={t.learning_rate} {t.optimizer} batch={t.batch_size}
                    {t.width_scale !== 1 ? ` ×${t.width_scale}w` : ''} →{' '}
                    {t.score === null ? 'failed' : `loss ${t.score.toFixed(4)}`}
                  </span>
                  {isBest && <span className="bb-chip bb-chip--accent">✓ applied</span>}
                </li>
              );
            })}
          </ul>
        )}
      </div>
    </Panel>
  );
}
