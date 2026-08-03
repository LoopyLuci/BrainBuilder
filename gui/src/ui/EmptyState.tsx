import { Button } from './Button';

interface EmptyStateProps {
  title?: string;
  description?: string;
  actionLabel?: string;
  onAction?: () => void;
}

export function EmptyState({ title = 'Nothing here yet', description, actionLabel, onAction }: EmptyStateProps) {
  return (
    <div className="bb-empty" style={{ padding: 'var(--space-4) 0' }}>
      <div style={{ fontSize: 13, fontWeight: 600 }}>{title}</div>
      {description && <div style={{ marginTop: 4, fontSize: 12 }}>{description}</div>}
      {actionLabel && onAction && (
        <div style={{ marginTop: 8 }}>
          <Button variant="secondary" onClick={onAction}>{actionLabel}</Button>
        </div>
      )}
    </div>
  );
}
