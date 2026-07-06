import { useState } from 'react';
import { synthesizeComponent, installSynthesizedComponent, SynthesisResult } from '../api/models';
import { getComponentDescriptors } from '../api/tauri';
import { useGraphStore } from '../state/graphStore';
import { useProviderStore } from '../state/providerStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { ProviderSelector } from '../llm/ProviderSelector';

// Create a brand-new component from a description. The backend runs the full
// validation gauntlet (parse → structure → sandboxed smoke test) and returns
// the generated descriptor + kernel + smoke result; only on the user's accept
// is it installed and hot-registered into the palette.
export function SynthesizePanel() {
  const [description, setDescription] = useState('');
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<SynthesisResult | null>(null);
  const selector = useProviderStore((s) => s.selector);
  const setDescriptors = useGraphStore((s) => s.setDescriptors);

  const synthesize = async () => {
    if (!description.trim()) return;
    setBusy(true);
    setResult(null);
    try {
      const r = await synthesizeComponent(description, selector());
      setResult(r);
      logInfo(
        r.smoke.passed
          ? `Synthesized "${r.name}" — smoke test passed (${JSON.stringify(r.smoke.actual_shape)}).`
          : `Synthesized "${r.name}" — smoke test FAILED: ${r.smoke.detail}`,
      );
    } catch (e) {
      logError(`Component synthesis failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const accept = async () => {
    if (!result) return;
    setBusy(true);
    try {
      await installSynthesizedComponent(result);
      // Refresh the palette so the new component appears live.
      setDescriptors(await getComponentDescriptors());
      logInfo(`Installed "${result.name}" — it's now in the Components palette.`);
      setResult(null);
      setDescription('');
    } catch (e) {
      logError(`Installing "${result.name}" failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Synthesize a Component" subtitle="Describe a new layer — it's generated, sandbox-tested, then added.">
      <textarea
        className="bb-textarea"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="e.g. a swish activation: x * sigmoid(x), same shape in and out"
        rows={3}
      />
      <ProviderSelector />
      <Button variant="primary" onClick={synthesize} disabled={busy || !description.trim()}>
        {busy ? 'Working…' : 'Synthesize'}
      </Button>

      {result && (
        <div style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span className={`bb-chip ${result.smoke.passed ? 'bb-chip--accent' : ''}`}>
              {result.smoke.passed ? 'Smoke test passed' : 'Smoke test failed'}
            </span>
            <span className="bb-text-muted" style={{ fontSize: 11 }}>
              {result.name}
            </span>
          </div>
          {!result.smoke.passed && <div className="bb-text-error" style={{ fontSize: 11 }}>{result.smoke.detail}</div>}
          <pre
            style={{
              maxHeight: 120,
              overflow: 'auto',
              margin: 0,
              fontFamily: 'var(--font-mono)',
              fontSize: 11,
              background: 'var(--surface-2, rgba(0,0,0,0.04))',
              padding: 8,
              borderRadius: 4,
            }}
          >
            {result.python_code}
          </pre>
          <Button variant="primary" onClick={accept} disabled={busy || !result.smoke.passed}>
            {result.smoke.passed ? 'Add to palette' : 'Cannot add (smoke test failed)'}
          </Button>
        </div>
      )}
    </Panel>
  );
}
