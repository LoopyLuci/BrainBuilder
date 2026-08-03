interface MetricCardProps {
  label: string;
  value: React.ReactNode;
  hint?: string;
}

export function MetricCard({ label, value, hint }: MetricCardProps) {
  return (
    <div className="bb-card" style={{ padding: 10, minHeight: 64 }}>
      <div className="bb-text-muted" style={{ fontSize: 11, marginBottom: 4 }}>{label}</div>
      <div style={{ fontSize: 18, fontWeight: 700, lineHeight: '20px' }}>{value}</div>
      {hint && <div className="bb-text-muted" style={{ fontSize: 10, marginTop: 2 }}>{hint}</div>}
    </div>
  );
}
