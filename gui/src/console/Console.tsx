import { useEffect, useState } from 'react';
import { useLogStore } from './logStore';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { Tabs } from '../ui/Tabs';
import { getNervousSystemAudit, NervousSystemAuditRecord } from '../api/tauri';

const logBoxStyle: React.CSSProperties = {
  fontFamily: 'var(--font-mono)',
  fontSize: 11,
  background: 'var(--bg-subtle)',
  border: '1px solid var(--border-subtle)',
  borderRadius: 'var(--radius-sm)',
  padding: 8,
  maxHeight: 180,
  overflowY: 'auto',
};

function OutputLog() {
  const entries = useLogStore((s) => s.entries);
  const clear = useLogStore((s) => s.clear);
  return (
    <div>
      <div style={{ display: 'flex', justifyContent: 'flex-end', marginBottom: 6 }}>
        <Button variant="ghost" data-tutorial="console-clear-btn" onClick={clear} disabled={entries.length === 0}>
          Clear
        </Button>
      </div>
      <div data-tutorial="console-output" style={logBoxStyle}>
        {entries.length === 0 ? (
          <div className="bb-empty">Output / logs appear here</div>
        ) : (
          entries.map((e) => (
            <div key={e.id} className={e.level === 'error' ? 'bb-text-error' : undefined} style={{ whiteSpace: 'pre-wrap', color: e.level === 'error' ? undefined : 'var(--text-secondary)' }}>
              [{new Date(e.timestamp).toLocaleTimeString()}] {e.message}
            </div>
          ))
        )}
      </div>
    </div>
  );
}

const OUTCOME_COLOR: Record<string, string> = {
  allowed: 'var(--success)',
  capability_denied: 'var(--danger)',
  timeout_killed: 'var(--danger)',
  worker_crashed: 'var(--warning)',
  allowed_but_failed: 'var(--warning)',
};

// Surfaces the nervous system's real audit trail: every sandboxed
// subprocess invocation (Racket/Clojure via Supervisor, Python via its
// persistent worker), whether it was allowed, denied by a capability check,
// or killed for exceeding its timeout. Polled rather than event-pushed
// since these are low-frequency, ad-hoc calls, not a training hot loop.
function NervousSystemAudit() {
  const [records, setRecords] = useState<NervousSystemAuditRecord[]>([]);

  useEffect(() => {
    let cancelled = false;
    const poll = () => {
      getNervousSystemAudit(100)
        .then((r) => {
          if (!cancelled) setRecords(r);
        })
        .catch(() => {});
    };
    poll();
    const id = setInterval(poll, 3000);
    return () => {
      cancelled = true;
      clearInterval(id);
    };
  }, []);

  return (
    <div data-tutorial="console-audit" style={logBoxStyle}>
      {records.length === 0 ? (
        <div className="bb-empty">No sandboxed component calls yet</div>
      ) : (
        records.map((r, i) => (
          <div key={i} style={{ whiteSpace: 'pre-wrap', marginBottom: 2 }}>
            <span style={{ color: 'var(--text-tertiary)' }}>[{new Date(r.logged_at).toLocaleTimeString()}]</span>{' '}
            <span style={{ color: 'var(--accent)' }}>{r.runtime}</span>{' '}
            <span style={{ color: OUTCOME_COLOR[r.outcome] ?? 'var(--text-secondary)' }}>{r.outcome}</span>
            {r.duration_ms != null && <span style={{ color: 'var(--text-tertiary)' }}> ({r.duration_ms}ms)</span>}
            {' — '}
            <span style={{ color: 'var(--text-secondary)' }}>{r.detail}</span>
          </div>
        ))
      )}
    </div>
  );
}

export function Console() {
  return (
    <Panel title="Console">
      <Tabs
        tabs={[
          { id: 'output', label: 'Output', content: <OutputLog /> },
          { id: 'audit', label: 'Nervous System', content: <NervousSystemAudit /> },
        ]}
      />
    </Panel>
  );
}
