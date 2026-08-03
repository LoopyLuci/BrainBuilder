import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/tauri';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { logError, logInfo } from '../console/logStore';

export interface SymbolicStepView {
  operation: string;
  result: string;
}

export function SymbolicReasoningPanel() {
  const [programJson, setProgramJson] = useState('{"steps": []}');
  const [operation, setOperation] = useState('assert');
  const [argsJson, setArgsJson] = useState('[]');
  const [trace, setTrace] = useState<SymbolicStepView[]>([]);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const execute = async () => {
    setBusy(true);
    setError(null);
    setTrace([]);
    try {
      const t = await invoke<SymbolicStepView[]>('symbolic_execute', { program_json: programJson });
      setTrace(t);
      logInfo(`symbolic_execute: ${t.length} steps`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`symbolic_execute failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  const addStep = async () => {
    setBusy(true);
    setError(null);
    try {
      await invoke('symbolic_add_step', { operation, args_json: argsJson });
      logInfo(`symbolic_add_step: ${operation}`);
    } catch (e) {
      const msg = String(e);
      setError(msg);
      logError(`symbolic_add_step failed: ${msg}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Symbolic Reasoning" subtitle="Step-by-step symbolic program execution.">
      {error && <div className="bb-text-error" style={{ marginBottom: 8 }}>{error}</div>}
      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Program JSON</div>
        <textarea
          className="bb-input"
          value={programJson}
          onChange={(e) => setProgramJson(e.target.value)}
          style={{ width: '100%', minHeight: 60, fontFamily: 'var(--font-mono)', fontSize: 12, marginBottom: 6 }}
        />
        <div className="bb-row">
          <Button variant="primary" onClick={execute} disabled={busy}>
            Execute
          </Button>
        </div>
      </div>

      <div style={{ marginBottom: 12 }}>
        <div className="bb-label" style={{ marginBottom: 4 }}>Add step</div>
        <div className="bb-row">
          <input
            className="bb-input"
            value={operation}
            onChange={(e) => setOperation(e.target.value)}
            style={{ flex: 1 }}
          />
          <input
            className="bb-input"
            value={argsJson}
            onChange={(e) => setArgsJson(e.target.value)}
            style={{ flex: 1 }}
          />
          <Button variant="ghost" onClick={addStep} disabled={busy}>
            Add
          </Button>
        </div>
      </div>

      {trace.length > 0 && (
        <div className="bb-card" style={{ marginTop: 8 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>Trace</div>
          <div style={{ maxHeight: 160, overflow: 'auto' }}>
            <table style={{ width: '100%', fontSize: 11, borderCollapse: 'collapse' }}>
              <thead>
                <tr style={{ borderBottom: '1px solid rgba(255,255,255,0.08)' }}>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Operation</th>
                  <th style={{ textAlign: 'left', padding: '4px 4px' }}>Result</th>
                </tr>
              </thead>
              <tbody>
                {trace.map((s, i) => (
                  <tr key={i} style={{ borderBottom: '1px solid rgba(255,255,255,0.04)' }}>
                    <td style={{ padding: '3px 4px', fontFamily: 'var(--font-mono)' }}>{s.operation}</td>
                    <td style={{ padding: '3px 4px' }}>{s.result}</td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </Panel>
  );
}
