import { Diagnostic } from '../api/tauri';

// Shared renderer for plain-English diagnostics (core/src/diagnostics.rs),
// used by both the data-time check (Intent panel) and the training-time check
// (Training dashboard). Each finding shows what's wrong, why, and what to do.

const ICON: Record<Diagnostic['severity'], string> = {
  error: '⛔',
  warning: '⚠️',
  info: '✅',
};

export function DiagnosticsList({ diagnostics }: { diagnostics: Diagnostic[] }) {
  if (diagnostics.length === 0) return null;
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      {diagnostics.map((d, i) => (
        <div key={i} className="bb-card" data-severity={d.severity}>
          <strong>
            {ICON[d.severity]} {d.title}
          </strong>
          <p className="bb-text-muted" style={{ margin: '4px 0 0' }}>{d.explanation}</p>
          <p style={{ margin: '4px 0 0' }}>
            <em>Try this:</em> {d.suggestion}
          </p>
        </div>
      ))}
    </div>
  );
}
