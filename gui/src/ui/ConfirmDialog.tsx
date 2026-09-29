import { useEffect, useRef } from 'react';
import { useConfirmStore } from './confirmStore';
import { Button } from './Button';

// Renders whatever confirmation request is currently queued in confirmStore.
// Mounted once near the app root. Escape/backdrop-click cancels (the safe
// default); the destructive action only ever fires from an explicit click on
// the confirm button, which is never auto-focused for `danger` requests so a
// stray Enter key-press can't trigger it.
export function ConfirmDialog() {
  const request = useConfirmStore((s) => s.request);
  const resolve = useConfirmStore((s) => s.resolve);
  const cancelRef = useRef<HTMLButtonElement>(null);

  useEffect(() => {
    if (!request) return;
    cancelRef.current?.focus();
    const onKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') resolve(false);
    };
    window.addEventListener('keydown', onKeyDown);
    return () => window.removeEventListener('keydown', onKeyDown);
  }, [request, resolve]);

  if (!request) return null;

  return (
    <div className="bb-confirm-scrim" role="presentation" onClick={() => resolve(false)}>
      <div
        className="bb-confirm-card"
        role="alertdialog"
        aria-modal="true"
        aria-labelledby="bb-confirm-title"
        onClick={(e) => e.stopPropagation()}
      >
        <h3 id="bb-confirm-title" className="bb-confirm-title">{request.title}</h3>
        <p className="bb-confirm-body">{request.body}</p>
        <div className="bb-confirm-actions">
          <Button ref={cancelRef} variant="secondary" onClick={() => resolve(false)}>Cancel</Button>
          <Button variant={request.danger ? 'danger' : 'primary'} onClick={() => resolve(true)}>
            {request.confirmLabel}
          </Button>
        </div>
      </div>
    </div>
  );
}
