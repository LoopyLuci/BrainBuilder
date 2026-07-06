import { useEffect, useState } from 'react';
import {
  LocalModel,
  addCustomModelDir,
  getCustomModelDirs,
  inspectOnnx,
  inspectSafetensors,
  listLocalModels,
  registerGgufModel,
  removeCustomModelDir,
} from '../api/models';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

function findFile(model: LocalModel, ext: string): string | null {
  const f = model.files.find((f) => f.toLowerCase().endsWith(ext));
  return f ? `${model.snapshot_path}/${f}` : null;
}

const FORMAT_LABEL: Record<string, string> = {
  safe_tensors: 'safetensors',
  gguf: 'GGUF',
  onnx: 'ONNX',
  pytorch_bin: 'PyTorch',
};

export function ModelHub() {
  const [models, setModels] = useState<LocalModel[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [manifest, setManifest] = useState<[string, number[], string][] | null>(null);
  const [busy, setBusy] = useState(false);
  const [customDirs, setCustomDirs] = useState<string[]>(getCustomModelDirs());
  const [newDir, setNewDir] = useState('');

  const refresh = () => {
    listLocalModels()
      .then(setModels)
      .catch((e) => setError(String(e)));
  };

  useEffect(refresh, []);

  const addDir = () => {
    const dir = newDir.trim();
    if (!dir) return;
    setCustomDirs(addCustomModelDir(dir));
    setNewDir('');
    refresh();
  };

  const removeDir = (dir: string) => {
    setCustomDirs(removeCustomModelDir(dir));
    refresh();
  };

  const inspect = async (model: LocalModel) => {
    setManifest(null);
    setExpanded(model.repo_id);
    const stPath = findFile(model, '.safetensors');
    const onnxPath = findFile(model, '.onnx');
    try {
      if (stPath) {
        setManifest(await inspectSafetensors(stPath));
      } else if (onnxPath) {
        const [inputs, outputs] = await inspectOnnx(onnxPath);
        setManifest([
          ...inputs.map((i) => [`input: ${i.name}`, i.shape, i.dtype] as [string, number[], string]),
          ...outputs.map((o) => [`output: ${o.name}`, o.shape, o.dtype] as [string, number[], string]),
        ]);
      }
    } catch (e) {
      logError(`Inspecting ${model.repo_id} failed: ${e}`);
    }
  };

  const registerGguf = async (model: LocalModel) => {
    const gguf = findFile(model, '.gguf');
    if (!gguf) return;
    setBusy(true);
    try {
      const modelName = model.repo_id.replace('/', '-').toLowerCase();
      await registerGgufModel(gguf, modelName);
      logInfo(`Registered "${modelName}" with Ollama — chat with it via \`ollama run ${modelName}\`.`);
    } catch (e) {
      logError(`Registering ${model.repo_id} with Ollama failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel
      title="Model Hub"
      action={
        <Button variant="ghost" onClick={refresh}>
          Refresh
        </Button>
      }
      subtitle="Any format, no download, no network — scanned from local folders."
    >
      <div className="bb-row">
        <input
          className="bb-input"
          value={newDir}
          onChange={(e) => setNewDir(e.target.value)}
          onKeyDown={(e) => e.key === 'Enter' && addDir()}
          placeholder={String.raw`e.g. D:\Models\general`}
        />
        <Button variant="secondary" onClick={addDir} disabled={!newDir.trim()}>
          Add folder
        </Button>
      </div>
      {customDirs.length > 0 && (
        <ul className="bb-list">
          {customDirs.map((dir) => (
            <li key={dir} className="bb-list-item" style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <span className="bb-text-muted">{dir}</span>
              <Button variant="ghost" onClick={() => removeDir(dir)}>
                Remove
              </Button>
            </li>
          ))}
        </ul>
      )}

      {error && <div className="bb-text-error">{error}</div>}
      {models.length === 0 && !error && <div className="bb-empty">No locally cached models found.</div>}

      <ul className="bb-list">
        {models.map((m) => (
          <li key={m.repo_id} className="bb-list-item">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 6 }}>
              <div style={{ minWidth: 0 }}>
                <div style={{ fontWeight: 600, overflow: 'hidden', textOverflow: 'ellipsis' }}>{m.repo_id}</div>
                <div style={{ display: 'flex', gap: 4, marginTop: 2 }}>
                  {m.formats.map((f) => (
                    <span key={f} className="bb-chip bb-chip--accent">
                      {FORMAT_LABEL[f] ?? f}
                    </span>
                  ))}
                </div>
              </div>
              <div style={{ display: 'flex', gap: 4, flexShrink: 0 }}>
                <Button variant="secondary" onClick={() => inspect(m)}>
                  Inspect
                </Button>
                {m.formats.includes('gguf') && (
                  <Button variant="secondary" onClick={() => registerGguf(m)} disabled={busy}>
                    Register
                  </Button>
                )}
              </div>
            </div>
            {expanded === m.repo_id && manifest && (
              <div style={{ maxHeight: 140, overflow: 'auto', marginTop: 6, fontFamily: 'var(--font-mono)', fontSize: 11 }}>
                {manifest.map(([name, shape, dtype], i) => (
                  <div key={i} className="bb-text-muted">
                    {name} {JSON.stringify(shape)} {dtype}
                  </div>
                ))}
              </div>
            )}
          </li>
        ))}
      </ul>
    </Panel>
  );
}
