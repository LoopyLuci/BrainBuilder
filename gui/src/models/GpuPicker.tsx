import { useEffect, useState } from 'react';
import {
  GpuAdapterInfo,
  listGpuAdapters,
  probeGpuAdapter,
  getPreferredGpu,
  setPreferredGpu,
  setBackendPreferredGpu,
} from '../api/models';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

// GPU device picker: lists every adapter wgpu can drive on this machine
// (Vulkan/DX12/Metal/GL), lets the user pin a preferred one (e.g. an AMD RX
// 7900 XTX over the integrated GPU), and probes it live to confirm the card
// actually binds. The preference persists in localStorage and is consumed by
// the wgpu backend's `with_preferred` path.
export function GpuPicker() {
  const [adapters, setAdapters] = useState<GpuAdapterInfo[] | null>(null);
  const [preferred, setPreferred] = useState<string>(getPreferredGpu());
  const [probe, setProbe] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listGpuAdapters()
      .then(setAdapters)
      .catch((e) => setError(String(e)));
  }, []);

  const choose = (name: string) => {
    setPreferred(name);
    setPreferredGpu(name); // persist app-side
    // Push to the backend so native ops route to it on the next run.
    setBackendPreferredGpu(name).catch((e) => logError(`Couldn't apply GPU preference: ${e}`));
    logInfo(name ? `Preferred GPU set to "${name}".` : 'Preferred GPU cleared (auto-select).');
  };

  const testBind = async () => {
    setBusy(true);
    setProbe(null);
    try {
      const bound = await probeGpuAdapter(preferred);
      setProbe(bound);
      logInfo(`GPU bound successfully: ${bound}`);
    } catch (e) {
      setProbe(null);
      logError(`GPU probe failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="GPU" subtitle="Cross-platform compute (Vulkan/DX12/Metal). Pin a card and test it.">
      {error && <div className="bb-text-error" style={{ fontSize: 11 }}>{error}</div>}
      {adapters === null && !error && <div className="bb-text-muted">Detecting GPUs…</div>}
      {adapters && adapters.length === 0 && (
        <div className="bb-empty">No wgpu-compatible GPU found on this machine.</div>
      )}
      {adapters && adapters.length > 0 && (
        <>
          <label className="bb-label" data-tutorial="gpu-select">Preferred device</label>
          <select className="bb-select" value={preferred} onChange={(e) => choose(e.target.value)}>
            <option value="">Auto (highest performance)</option>
            {adapters.map((a, i) => (
              <option key={`${a.name}-${a.backend}-${i}`} value={a.name}>
                {a.name} · {a.backend} · {a.device_type}
              </option>
            ))}
          </select>
          <Button variant="secondary" data-tutorial="gpu-test-btn" onClick={testBind} disabled={busy}>
            {busy ? 'Binding…' : 'Test this GPU'}
          </Button>
          {probe && (
            <div style={{ marginTop: 6 }} data-tutorial="gpu-probe-result">
              <span className="bb-chip bb-chip--accent">bound</span>{' '}
              <span className="bb-text-muted" style={{ fontSize: 11 }}>{probe}</span>
            </div>
          )}
        </>
      )}
    </Panel>
  );
}
