import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface CompositionResultView {
  program: string;
  output: string;
}

export function CompositionalReasoningPanel() {
  const [name, setName] = useState('');
  const [steps, setSteps] = useState('[]');
  const [input, setInput] = useState('');
  const [result, setResult] = useState<CompositionResultView | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const compose = async () => {
    if (!name.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const stepsArr = JSON.parse(steps) as string[];
      await invoke('compositional_compose', { name: name.trim(), steps: stepsArr, repeatable: false });
      logInfo(`compositional_compose: ${name}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`compositional_compose failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const executeProgram = async () => {
    if (!name.trim() || !input.trim()) return;
    setBusy(true);
    setError(null);
    setResult(null);
    try {
      const r = await invoke<CompositionResultView>('compositional_execute', {
        program_name: name.trim(),
        input,
      });
      setResult(r);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`compositional_execute failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Compositional Reasoning" subtitle="Compose and execute reusable reasoning programs.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Compose</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={name}
            onChange={(e) => setName(e.target.value)}
            placeholder="Program name"
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            value={steps}
            onChange={(e) => setSteps(e.target.value)}
            placeholder='["step1","step2"]'
            style={{ flex: 1, fontFamily: 'var(--font-mono)', fontSize: 12 }}
          />
          <Button variant="primary" onClick={compose} disabled={busy || !name.trim()}>
            Compose
          </Button>
        </div>
      </div>

      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Execute</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={input}
            onChange={(e) => setInput(e.target.value)}
            placeholder="Input"
            style={{ flex: 1 }}
          />
          <Button variant="ghost" onClick={executeProgram} disabled={busy || !name.trim() || !input.trim()}>
            Run
          </Button>
        </div>
      </div>

      {result && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div style={{ fontSize: 12 }}>
            <span className="bb-text-muted">Program: </span>
            <span style={{ fontWeight: 600 }}>{result.program}</span>
          </div>
          <div style={{ fontSize: 12, marginTop: 4, fontFamily: 'var(--font-mono)' }}>{result.output}</div>
        </div>
      )}
    </Panel>
  );
}
