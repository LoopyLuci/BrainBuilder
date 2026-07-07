import { useMemo } from 'react';
import { useGraphStore } from '../state/graphStore';
import { TEMPLATES, missingComponents, Template } from './registry';
import { useApplyTemplate } from './useApplyTemplate';
import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';

// One-click presets. Picking a template lays out its whole component graph on
// the canvas (with settings applied) so a user starts from a working
// architecture — e.g. a DeepSeek-style speculative drafter — and customizes.
export function TemplatesPanel() {
  const descriptors = useGraphStore((s) => s.descriptors);
  const use = useApplyTemplate();

  // Group templates by category for a tidy list.
  const grouped = useMemo(() => {
    const by: Record<string, Template[]> = {};
    for (const t of TEMPLATES) (by[t.category] ??= []).push(t);
    return by;
  }, []);

  return (
    <Panel title="Templates" subtitle="Start from a preset architecture, then customize.">
      {Object.entries(grouped).map(([category, templates]) => (
        <div key={category} style={{ marginBottom: 10 }}>
          <div className="bb-label" style={{ marginBottom: 4 }}>{category}</div>
          <ul className="bb-list">
            {templates.map((t) => {
              const missing = missingComponents(t, descriptors);
              return (
                <li
                  key={t.id}
                  className="bb-list-item"
                  style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'flex-start', gap: 8 }}
                >
                  <div style={{ minWidth: 0 }}>
                    <div style={{ fontWeight: 600 }}>{t.name}</div>
                    <div className="bb-text-muted" style={{ fontSize: 11 }}>{t.description}</div>
                    {missing.length > 0 && (
                      <div className="bb-text-error" style={{ fontSize: 10 }}>
                        missing: {missing.join(', ')}
                      </div>
                    )}
                  </div>
                  <Button variant="secondary" onClick={() => use(t)} disabled={missing.length > 0}>
                    Use
                  </Button>
                </li>
              );
            })}
          </ul>
        </div>
      ))}
    </Panel>
  );
}
