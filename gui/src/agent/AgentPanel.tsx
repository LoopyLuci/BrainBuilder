import { useEffect, useState } from 'react';
import { listen } from '@tauri-apps/api/event';
import {
  AgentSession,
  AgentRunReport,
  AgentProgress,
  AutonomyMode,
  agentApprove,
  agentRevert,
  agentRun,
  agentStart,
  agentStatus,
} from './api';
import { useProviderStore } from '../state/providerStore';
import { logError, logInfo } from '../console/logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

const MODES: { id: AutonomyMode; label: string; blurb: string }[] = [
  { id: 'propose-approve', label: 'Propose + approve', blurb: 'Edits in an isolated worktree; you approve the diff before it merges.' },
  { id: 'auto-apply', label: 'Auto-apply', blurb: 'Edits + tests + merges automatically on green; revert via git if unhappy.' },
  { id: 'full', label: 'Full autonomy', blurb: 'Edits + merges with no gate. Maximum power, opt-in.' },
];

// The self-building agent: BrainBuilder driving OpenCode against its own repo,
// inside a sandboxed git worktree, at a user-chosen autonomy level.
export function AgentPanel() {
  const [session, setSession] = useState<AgentSession | null>(null);
  const [mode, setMode] = useState<AutonomyMode>('propose-approve');
  const [task, setTask] = useState('');
  const [busy, setBusy] = useState(false);
  // Partial while streaming; the final agentRun result overwrites it whole.
  const [report, setReport] = useState<Partial<AgentRunReport> | null>(null);
  const [progress, setProgress] = useState<string | null>(null);
  const selector = useProviderStore((s) => s.selector);

  useEffect(() => {
    agentStatus().then(setSession).catch(() => setSession(null));
  }, []);

  // Subscribe to streamed phase updates so the panel fills in live: status
  // line, the diff as soon as it's computed, then the gate result.
  useEffect(() => {
    const un = listen<AgentProgress>('agent-progress', (e) => {
      const p = e.payload;
      switch (p.phase) {
        case 'agent':
          setProgress(p.message ?? 'Working…');
          break;
        case 'agent-output':
          setReport((r) => ({ ...r, agent_output: p.message }));
          break;
        case 'diff':
          setProgress('Reviewing the diff…');
          setReport((r) => ({ ...r, diff: p.diff }));
          break;
        case 'tests':
          setProgress(p.message ?? 'Running tests…');
          break;
        case 'gate':
          setReport((r) => ({ ...r, gate_passed: p.gate_passed, gate_output: p.gate_output }));
          break;
        case 'done':
          setProgress(null);
          setReport((r) => ({ ...r, merged: p.merged }));
          break;
      }
    });
    return () => {
      un.then((f) => f());
    };
  }, []);

  const start = async () => {
    setBusy(true);
    try {
      setSession(await agentStart(mode));
      logInfo(`Agent session started (${mode}) in an isolated worktree.`);
    } catch (e) {
      logError(`Starting agent failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const run = async () => {
    if (!task.trim()) return;
    setBusy(true);
    setReport(null);
    setProgress('Starting…');
    try {
      const r = await agentRun(task, selector());
      setReport(r); // authoritative final report
      logInfo(`Agent step done — tests ${r.gate_passed ? 'passed' : 'failed'}, ${r.merged ? 'merged' : 'not merged'}.`);
    } catch (e) {
      logError(`Agent run failed: ${e}`);
    } finally {
      setProgress(null);
      setBusy(false);
    }
  };

  const approve = async () => {
    setBusy(true);
    try {
      await agentApprove();
      logInfo('Agent changes merged into your checkout.');
    } catch (e) {
      logError(`Approve failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  const revert = async () => {
    setBusy(true);
    try {
      await agentRevert();
      setSession(null);
      setReport(null);
      logInfo('Agent worktree discarded — your checkout is untouched.');
    } catch (e) {
      logError(`Revert failed: ${e}`);
    } finally {
      setBusy(false);
    }
  };

  return (
    <Panel title="Self-Building Agent" subtitle="OpenCode works on BrainBuilder inside a sandboxed git worktree.">
      {!session ? (
        <>
          <label className="bb-label">Autonomy</label>
          <select className="bb-select" value={mode} onChange={(e) => setMode(e.target.value as AutonomyMode)}>
            {MODES.map((m) => (
              <option key={m.id} value={m.id}>
                {m.label}
              </option>
            ))}
          </select>
          <p className="bb-text-muted" style={{ margin: 0, fontSize: 11 }}>
            {MODES.find((m) => m.id === mode)?.blurb}
          </p>
          <Button variant="primary" onClick={start} disabled={busy}>
            Start session
          </Button>
        </>
      ) : (
        <>
          <div className="bb-text-muted" style={{ fontSize: 11 }}>
            <span className="bb-chip bb-chip--accent">{session.mode}</span> branch <code className="bb-code">{session.branch}</code>
          </div>
          <textarea
            className="bb-textarea"
            value={task}
            onChange={(e) => setTask(e.target.value)}
            placeholder="e.g. add a `swish` activation component with a smoke test"
            rows={2}
          />
          <div className="bb-row">
            <Button variant="primary" onClick={run} disabled={busy || !task.trim()}>
              {busy ? 'Running…' : 'Run task'}
            </Button>
            <Button variant="secondary" onClick={approve} disabled={busy}>
              Approve + merge
            </Button>
            <Button variant="ghost" onClick={revert} disabled={busy}>
              Discard
            </Button>
          </div>

          {progress && (
            <div className="bb-text-muted" style={{ fontSize: 11, display: 'flex', alignItems: 'center', gap: 6 }}>
              <span className="bb-chip">running</span>
              {progress}
            </div>
          )}

          {report && (
            <div style={{ display: 'flex', flexDirection: 'column', gap: 6, marginTop: 6 }}>
              <div style={{ display: 'flex', gap: 6, alignItems: 'center' }}>
                {report.gate_passed === undefined ? (
                  <span className="bb-chip">tests pending</span>
                ) : (
                  <span className={`bb-chip ${report.gate_passed ? 'bb-chip--accent' : ''}`}>
                    tests {report.gate_passed ? 'passed' : 'failed'}
                  </span>
                )}
                {report.merged && <span className="bb-chip bb-chip--accent">merged</span>}
              </div>
              {report.diff && (
                <pre style={preStyle}>{report.diff}</pre>
              )}
              {report.gate_output && !report.gate_passed && report.gate_passed !== undefined && (
                <pre style={preStyle}>{report.gate_output}</pre>
              )}
              {report.agent_output && <pre style={preStyle}>{report.agent_output}</pre>}
            </div>
          )}
        </>
      )}
    </Panel>
  );
}

const preStyle: React.CSSProperties = {
  maxHeight: 160,
  overflow: 'auto',
  margin: 0,
  fontFamily: 'var(--font-mono)',
  fontSize: 11,
  background: 'var(--surface-2, rgba(0,0,0,0.04))',
  padding: 8,
  borderRadius: 4,
  whiteSpace: 'pre-wrap',
};
