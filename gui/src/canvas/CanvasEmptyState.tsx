import { useGraphStore } from '../state/graphStore';
import { TEMPLATES, missingComponents } from '../templates/registry';
import { useApplyTemplate } from '../templates/useApplyTemplate';
import { Button } from '../ui/Button';

// Shown centered over an empty canvas so a new user has an obvious first move:
// start from a preset architecture (including the DSpark drafter) rather than a
// blank page. Uses the same apply-template action as the Templates panel.
export function CanvasEmptyState() {
  const descriptors = useGraphStore((s) => s.descriptors);
  const apply = useApplyTemplate();

  return (
    <div className="bb-canvas-empty">
      <div className="bb-canvas-empty__card">
        <div className="bb-canvas-empty__title">Start from a template</div>
        <div className="bb-text-muted" style={{ fontSize: 12, marginBottom: 10 }}>
          Pick a preset to lay out a working architecture, then customize — or drag components
          from the palette to build from scratch.
        </div>
        <ul className="bb-list">
          {TEMPLATES.map((t) => {
            const missing = missingComponents(t, descriptors);
            return (
              <li
                key={t.id}
                className="bb-list-item"
                style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', gap: 8 }}
              >
                <div style={{ minWidth: 0 }}>
                  <div style={{ fontWeight: 600 }}>{t.name}</div>
                  <div className="bb-text-muted" style={{ fontSize: 11 }}>{t.category}</div>
                </div>
                <Button variant="secondary" onClick={() => apply(t)} disabled={missing.length > 0}>
                  Use
                </Button>
              </li>
            );
          })}
        </ul>
      </div>
    </div>
  );
}
