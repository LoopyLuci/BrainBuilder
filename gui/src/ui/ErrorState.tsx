import { Button } from './Button';

interface ErrorStateProps {
  title?: string;
  message?: string;
  retryLabel?: string;
  onRetry?: () => void;
}

export function ErrorState({ title = 'Something went wrong', message, retryLabel = 'Retry', onRetry }: ErrorStateProps) {
  return (
    <div className="bb-empty" style={{ color: 'var(--danger)' }}>
      <div style={{ fontSize: 13, fontWeight: 600 }}>{title}</div>
      {message && <div style={{ marginTop: 4, fontSize: 12, whiteSpace: 'pre-wrap' }}>{message}</div>}
      {onRetry && (
        <div style={{ marginTop: 8 }}>
          <Button variant="secondary" onClick={onRetry}>{retryLabel}</Button>
        </div>
      )}
    </div>
  );
}
