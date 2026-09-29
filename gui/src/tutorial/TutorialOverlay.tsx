import { useEffect, useState } from 'react';
import { useTutorialStore } from './tutorialStore';
import { useLayoutStore } from '../state/layoutStore';

interface Rect {
  top: number;
  left: number;
  width: number;
  height: number;
}

// Renders a darkened overlay with a cut-out "spotlight" around the current
// step's real target element, plus an instruction card. Mounted once, near
// the root of the app (outside any widget slot), so it can sit above
// everything. Recomputes the spotlight rect on resize/scroll and re-measures
// a short beat after a tab switch (forceActive), since the target may not be
// laid out yet the instant the tab becomes active.
export function TutorialOverlay() {
  const activeId = useTutorialStore((s) => s.activeId);
  const stepIndex = useTutorialStore((s) => s.stepIndex);
  const tutorial = useTutorialStore((s) => s.activeTutorial());
  const next = useTutorialStore((s) => s.next);
  const prev = useTutorialStore((s) => s.prev);
  const exit = useTutorialStore((s) => s.exit);
  const showWidget = useLayoutStore((s) => s.show);

  const step = tutorial?.steps[stepIndex];
  const [rect, setRect] = useState<Rect | null>(null);

  // Make sure the target widget isn't hidden by the user before we try to
  // spotlight it — a tutorial step must never point at something invisible.
  useEffect(() => {
    if (!step) return;
    if (step.focusTab) showWidget(step.focusTab.tabId);
  }, [step, showWidget]);

  useEffect(() => {
    if (!step?.target) {
      setRect(null);
      return;
    }
    let cancelled = false;
    const measure = () => {
      if (cancelled) return;
      const el = document.querySelector(step.target!);
      if (!el) {
        setRect(null);
        return;
      }
      const r = el.getBoundingClientRect();
      setRect({ top: r.top, left: r.left, width: r.width, height: r.height });
    };
    // Tab switches / layout re-renders happen async; retry a few times over
    // the next half-second rather than measuring once too early.
    measure();
    const timers = [50, 150, 300, 500].map((ms) => setTimeout(measure, ms));
    window.addEventListener('resize', measure);
    window.addEventListener('scroll', measure, true);
    return () => {
      cancelled = true;
      timers.forEach(clearTimeout);
      window.removeEventListener('resize', measure);
      window.removeEventListener('scroll', measure, true);
    };
  }, [step]);

  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if (!activeId) return;
      if (e.key === 'Escape') exit();
      if (e.key === 'ArrowRight') next();
      if (e.key === 'ArrowLeft') prev();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [activeId, next, prev, exit]);

  if (!tutorial || !step) return null;

  const pad = 8;
  const spotlight = rect
    ? { top: rect.top - pad, left: rect.left - pad, width: rect.width + pad * 2, height: rect.height + pad * 2 }
    : null;

  // Position the card near the spotlight, clamped to the viewport; centered
  // when there's no target (intro/outro steps).
  const cardStyle: React.CSSProperties = spotlight
    ? {
        top: Math.min(Math.max(spotlight.top + spotlight.height + 12, 12), window.innerHeight - 220),
        left: Math.min(Math.max(spotlight.left, 12), window.innerWidth - 360),
      }
    : { top: '40%', left: '50%', transform: 'translate(-50%, -50%)' };

  return (
    <div className="bb-tutorial-overlay" role="dialog" aria-label={`${tutorial.title} tutorial`}>
      <svg className="bb-tutorial-overlay__scrim" width="100%" height="100%">
        <defs>
          <mask id="bb-tutorial-mask">
            <rect width="100%" height="100%" fill="white" />
            {spotlight && (
              <rect
                x={spotlight.left}
                y={spotlight.top}
                width={spotlight.width}
                height={spotlight.height}
                rx={10}
                fill="black"
              />
            )}
          </mask>
        </defs>
        <rect width="100%" height="100%" fill="rgba(10, 12, 16, 0.6)" mask="url(#bb-tutorial-mask)" />
      </svg>
      {spotlight && (
        <div
          className="bb-tutorial-overlay__ring"
          style={{ top: spotlight.top, left: spotlight.left, width: spotlight.width, height: spotlight.height }}
        />
      )}

      <div className="bb-tutorial-card" style={cardStyle}>
        <div className="bb-tutorial-card__progress">
          Step {stepIndex + 1} of {tutorial.steps.length}
        </div>
        <h3 className="bb-tutorial-card__title">{step.title}</h3>
        <p className="bb-tutorial-card__body">{step.body}</p>
        <div className="bb-tutorial-card__actions">
          <button className="bb-btn bb-btn--ghost bb-btn--sm" onClick={exit}>
            Exit tour
          </button>
          <div style={{ flex: 1 }} />
          {stepIndex > 0 && (
            <button className="bb-btn bb-btn--secondary bb-btn--sm" onClick={prev}>
              Back
            </button>
          )}
          <button className="bb-btn bb-btn--primary bb-btn--sm" onClick={next}>
            {stepIndex + 1 >= tutorial.steps.length ? 'Finish' : 'Next'}
          </button>
        </div>
      </div>
    </div>
  );
}
