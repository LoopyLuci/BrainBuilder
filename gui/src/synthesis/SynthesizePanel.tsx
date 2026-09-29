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
  const addNode = useGraphStore((s) => s.addNode);

  const synthesize = async () => {
    if (!description.trim()) return;
    setBusy(true);
    setResult(null);
    try {
      const r = await synthesizeComponent(description, selector());
      setResult(r);
      const repaired = (r.attempts ?? 1) > 1 ? ' (auto-repaired)' : '';
      logInfo(
        r.smoke.passed
          ? `Synthesized "${r.name}"${repaired} — smoke test passed (${JSON.stringify(r.smoke.actual_shape)}).`
          : `Synthesized "${r.name}"${repaired} — smoke test FAILED: ${r.smoke.detail}`,
      );
    } catch (e) {
      logError(`Component synthesis failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  // Install the synthesized component and (when `place`) immediately drop it on
  // the canvas — so a user sees describe → smoke-green → node-on-canvas without
  // ever hunting through the palette or knowing EDN exists.
  const accept = async (place: boolean) => {
    if (!result) return;
    const name = result.name;
    setBusy(true);
    try {
      await installSynthesizedComponent(result);
      // Refresh the palette so the new component appears live. addNode below
      // reads the descriptor from this same store, so it must run first.
      setDescriptors(await getComponentDescriptors());
      if (place) {
        // Drop near the top-left of the canvas; the user can drag from there.
        addNode(name, { x: 120, y: 120 });
        logInfo(`Installed "${name}" and placed it on the canvas.`);
      } else {
        logInfo(`Installed "${name}" — it's now in the Components palette.`);
      }
      setResult(null);
      setDescription('');
    } catch (e) {
      logError(`Installing "${name}" failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Synthesize a Component" subtitle="Describe a new layer — it's generated, sandbox-tested, then added.">
      <textarea
        className="bb-textarea"
        data-tutorial="synth-description"
        value={description}
        onChange={(e) => setDescription(e.target.value)}
        placeholder="e.g. a swish activation: x * sigmoid(x), same shape in and out"
        rows={3}
      />
      <ProviderSelector />
      <Button variant="primary" data-tutorial="synth-button" onClick={synthesize} disabled={busy || !description.trim()}>
        {busy ? 'Working…' : 'Synthesize'}
      </Button>

      {result && (
        <div data-tutorial="synth-result" style={{ marginTop: 8, display: 'flex', flexDirection: 'column', gap: 6 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
            <span className={`bb-chip ${result.smoke.passed ? 'bb-chip--accent' : ''}`}>
              {result.smoke.passed ? 'Smoke test passed' : 'Smoke test failed'}
            </span>
            <span className="bb-text-muted" style={{ fontSize: 11 }}>
              {result.name}
            </span>
            {(result.attempts ?? 1) > 1 && <span className="bb-chip">auto-repaired</span>}
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
          <div style={{ display: 'flex', gap: 6 }}>
            <Button
              variant="primary"
              data-tutorial="synth-accept-btn"
              onClick={() => accept(true)}
              disabled={busy || !result.smoke.passed}
            >
              {result.smoke.passed ? 'Add to canvas' : 'Cannot add (smoke test failed)'}
            </Button>
            {result.smoke.passed && (
              <Button variant="secondary" onClick={() => accept(false)} disabled={busy}>
                Palette only
              </Button>
            )}
          </div>
        </div>
      )}
    </Panel>
  );
}
