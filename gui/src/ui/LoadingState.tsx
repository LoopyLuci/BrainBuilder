interface LoadingStateProps {
  label?: string;
}

export function LoadingState({ label = 'Loading…' }: LoadingStateProps) {
  return (
    <div className="bb-empty" aria-live="polite" aria-busy="true">
      <div style={{ fontSize: 13, fontWeight: 600 }}>{label}</div>
      <div style={{ marginTop: 6, height: 6, width: 120, borderRadius: 999, background: 'var(--bg-subtle)', overflow: 'hidden' }}>
        <div style={{ height: '100%', width: '40%', borderRadius: 999, background: 'var(--accent)', animation: 'bb-pulse 1s infinite' }} />
      </div>
      <style>{`
        @keyframes bb-pulse {
          0% { transform: translateX(-100%); }
          100% { transform: translateX(260%); }
        }
      `}</style>
    </div>
  );
}
