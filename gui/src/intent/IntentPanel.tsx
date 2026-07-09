import { useState } from 'react';
import { open } from '@tauri-apps/api/dialog';
import {
  proposeModel,
  proposeTransferModel,
  diagnoseData,
  getComponentDescriptors,
  Diagnostic,
  IntentRequest,
  ProposedModel,
  TaskKind,
} from '../api/tauri';
import { convertFromBBIR } from '../canvas/utils';
import { useGraphStore } from '../state/graphStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { DiagnosticsList } from './DiagnosticsList';
import { HelpTip } from '../help/HelpTip';

// The task-first on-ramp: describe your goal in plain terms and point at your
// data — BrainBuilder inspects the data, proposes a validated, trainable model
// sized to it, and drops it onto the canvas ready to train. No components, no
// graph editing, no hyperparameters required.

type SourceType = 'image_folder' | 'text_column' | 'file';

const SOURCE_LABELS: Record<SourceType, string> = {
  image_folder: 'A folder of images (labelled by subfolder)',
  text_column: 'A spreadsheet with a text column',
  file: 'A spreadsheet of numbers (CSV/Parquet)',
};

export function IntentPanel() {
  const [task, setTask] = useState<TaskKind>('classification');
  const [sourceType, setSourceType] = useState<SourceType>('image_folder');
  const [path, setPath] = useState('');
  const [imageSize, setImageSize] = useState(32);
  const [grayscale, setGrayscale] = useState(false);
  const [textColumn, setTextColumn] = useState('');
  const [labelColumn, setLabelColumn] = useState('');
  const [useTransfer, setUseTransfer] = useState(false);
  const [pretrainedFile, setPretrainedFile] = useState('');
  const [pretrainedTensor, setPretrainedTensor] = useState('');
  const [busy, setBusy] = useState(false);
  const [proposal, setProposal] = useState<ProposedModel | null>(null);
  const [diagnostics, setDiagnostics] = useState<Diagnostic[]>([]);
  const [error, setError] = useState<string | null>(null);

  const setGraph = useGraphStore((s) => s.setGraph);
  const setTraining = useGraphStore((s) => s.setTraining);

  const pick = async () => {
    // Image classification points at a folder; the others at a single file.
    const selected = await open(
      sourceType === 'image_folder'
        ? { directory: true, multiple: false }
        : { multiple: false, filters: [{ name: 'Dataset', extensions: ['csv', 'parquet'] }] },
    );
    if (typeof selected === 'string') {
      setPath(selected);
      setError(null);
    }
  };

  // What's stopping "Build my model" from being clickable right now, in plain
  // language — shown under the button so a stuck user always knows the exact
  // next thing to do, instead of a disabled button with no explanation.
  const blockedReason = !path.trim()
    ? `Choose ${sourceType === 'image_folder' ? 'a folder' : 'a file'} above first.`
    : sourceType === 'text_column' && !textColumn.trim()
    ? 'Type the name of the text column above first.'
    : useTransfer && (!pretrainedFile.trim() || !pretrainedTensor.trim())
    ? "You've turned on transfer learning — choose a pretrained model file and its tensor name, or turn transfer learning off."
    : null;

  const propose = async () => {
    if (blockedReason) return;
    setBusy(true);
    setError(null);
    setProposal(null);
    setDiagnostics([]);
    try {
      const data = {
        source_type: sourceType,
        path,
        image_size: sourceType === 'image_folder' ? imageSize : undefined,
        grayscale: sourceType === 'image_folder' ? grayscale : undefined,
        text_column: sourceType === 'text_column' ? textColumn || undefined : undefined,
        label_column: labelColumn || undefined,
      };
      const request: IntentRequest = { task, data };

      const result =
        useTransfer && pretrainedFile.trim() && pretrainedTensor.trim()
          ? await proposeTransferModel({
              task,
              data,
              pretrained_file: pretrainedFile,
              pretrained_tensor: pretrainedTensor,
            })
          : await proposeModel(request);
      setProposal(result);

      // Run data-time diagnostics in the background — never block the proposal
      // on them, but surface any imbalance/leakage warnings when they arrive.
      diagnoseData(request)
        .then(setDiagnostics)
        .catch((e) => logError(`Data diagnostics failed: ${e}`));

      // Drop the proposed graph straight onto the canvas, ready to train.
      const descriptors = Object.fromEntries((await getComponentDescriptors()).map((d) => [d.name, d]));
      const { nodes, edges } = convertFromBBIR(result.graph as any, descriptors);
      setGraph(nodes, edges, result.graph.graph_id);
      if (result.graph.training) setTraining(result.graph.training as any);
      logInfo(`Proposed "${result.graph.name}" (${nodes.length} nodes). ${result.rationale}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`Model proposal failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Build a Model from a Goal">
      <p className="bb-text-muted" style={{ margin: 0 }}>
        Tell BrainBuilder what you want and point at your data. It inspects the data, builds a model sized to it,
        and puts it on the canvas ready to train — no ML knowledge needed.
      </p>

      <label className="bb-label" data-tutorial="intent-task">I want to…</label>
      <select className="bb-input" value={task} onChange={(e) => setTask(e.target.value as TaskKind)}>
        <option value="classification">Sort my data into categories (classify)</option>
        <option value="regression">Predict a number</option>
      </select>
      <HelpTip term={task === 'regression' ? 'regression' : 'classification'} />

      <div data-tutorial="intent-data">
        <label className="bb-label">My data is…</label>
        <select
          className="bb-input"
          value={sourceType}
          onChange={(e) => {
            setSourceType(e.target.value as SourceType);
            setPath('');
            setProposal(null);
          }}
        >
          {(Object.keys(SOURCE_LABELS) as SourceType[]).map((s) => (
            <option key={s} value={s}>
              {SOURCE_LABELS[s]}
            </option>
          ))}
        </select>

        <div className="bb-row">
          <input className="bb-input" value={path} readOnly placeholder="Choose your data…" />
          <Button onClick={pick}>{sourceType === 'image_folder' ? 'Choose folder' : 'Choose file'}</Button>
        </div>
      </div>

      {sourceType === 'image_folder' && (
        <div className="bb-row">
          <label className="bb-label" style={{ margin: 0 }}>
            Resize to
            <input
              className="bb-input"
              type="number"
              min={8}
              max={128}
              value={imageSize}
              onChange={(e) => setImageSize(Math.max(8, Number(e.target.value) || 32))}
              style={{ width: 72, marginLeft: 8 }}
            />
            px
          </label>
          <label className="bb-label" style={{ margin: 0 }}>
            <input type="checkbox" checked={grayscale} onChange={(e) => setGrayscale(e.target.checked)} /> Grayscale
          </label>
        </div>
      )}

      {sourceType === 'text_column' && (
        <div className="bb-row">
          <input
            className="bb-input"
            value={textColumn}
            onChange={(e) => setTextColumn(e.target.value)}
            placeholder="Text column name (e.g. review) — required"
            aria-required="true"
            style={!textColumn.trim() ? { borderColor: 'var(--warning, #c90)' } : undefined}
          />
          <input
            className="bb-input"
            value={labelColumn}
            onChange={(e) => setLabelColumn(e.target.value)}
            placeholder="Label column (optional)"
          />
        </div>
      )}

      {sourceType === 'file' && (
        <input
          className="bb-input"
          value={labelColumn}
          onChange={(e) => setLabelColumn(e.target.value)}
          placeholder="Which column to predict (defaults to the last)"
        />
      )}

      <label className="bb-label" data-tutorial="intent-transfer" style={{ margin: '4px 0 0' }}>
        <input type="checkbox" checked={useTransfer} onChange={(e) => setUseTransfer(e.target.checked)} /> Start from a
        pretrained model (transfer learning) <HelpTip term="transfer-learning" />
      </label>
      {useTransfer && (
        <div style={{ display: 'flex', flexDirection: 'column', gap: 6 }}>
          <p className="bb-text-muted" style={{ margin: 0 }}>
            Freezes a pretrained backbone and trains a fresh head on top — learns fast on little data. The backbone's
            input size must match your data's features.
          </p>
          <div className="bb-row">
            <input className="bb-input" value={pretrainedFile} readOnly placeholder="Choose a .safetensors file…" />
            <Button
              onClick={async () => {
                const sel = await open({ multiple: false, filters: [{ name: 'Model', extensions: ['safetensors'] }] });
                if (typeof sel === 'string') setPretrainedFile(sel);
              }}
            >
              Choose model
            </Button>
          </div>
          <input
            className="bb-input"
            value={pretrainedTensor}
            onChange={(e) => setPretrainedTensor(e.target.value)}
            placeholder="Backbone weight tensor name (e.g. encoder.weight)"
          />
        </div>
      )}

      <Button
        variant="primary"
        data-tutorial="intent-build-btn"
        onClick={propose}
        disabled={busy || !!blockedReason}
        title={blockedReason ?? undefined}
      >
        {busy ? 'Inspecting your data…' : 'Build my model'}
      </Button>
      {!busy && blockedReason && (
        <p className="bb-text-muted" style={{ margin: 0, fontSize: 12 }}>{blockedReason}</p>
      )}

      {error && (
        <div className="bb-card" style={{ marginTop: 4, borderColor: 'var(--error, #d33)' }}>
          <p className="bb-text-error" style={{ margin: 0 }}>{error}</p>
          <p className="bb-text-muted" style={{ margin: '4px 0 0' }}>
            Nothing was changed — fix the data above and click "Build my model" again.
          </p>
        </div>
      )}

      {proposal && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <strong>{proposal.graph.name}</strong>
          <p className="bb-text-muted" style={{ marginTop: 4 }}>{proposal.rationale}</p>
          {proposal.class_names.length > 0 && (
            <p className="bb-text-muted" style={{ margin: 0 }}>
              Classes: {proposal.class_names.join(', ')}
            </p>
          )}
          <p className="bb-text-muted" style={{ margin: 0 }}>
            It's on the canvas now — switch to the Training tab and press Train.
          </p>
        </div>
      )}

      {diagnostics.length > 0 && (
        <div style={{ marginTop: 8 }}>
          <label className="bb-label">A few things to check about your data</label>
          <DiagnosticsList diagnostics={diagnostics} />
        </div>
      )}
    </Panel>
  );
}
