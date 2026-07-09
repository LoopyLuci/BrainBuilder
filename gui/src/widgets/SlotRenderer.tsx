import { useEffect, useRef, useState } from 'react';
import { useWidgetRegistry } from './registry';
import { useLayoutStore } from '../state/layoutStore';
import { WidgetBoundary } from './WidgetBoundary';
import { WidgetSlot } from './types';
import { Tabs, TabDef } from '../ui/Tabs';
import { useTutorialStore } from '../tutorial/tutorialStore';

// Renders every widget registered for a slot, each inside its own error
// boundary. `side`/`bottom` render as tabs (matching the previous shell);
// `palette`/`canvas`/`header` render their widgets stacked, since those slots
// are single-purpose today but stay pluggable.
export function SlotRenderer({ slot, asTabs }: { slot: WidgetSlot; asTabs?: boolean }) {
  // Subscribe to the widgets map so runtime (un)registration re-renders.
  const widgetsMap = useWidgetRegistry((s) => s.widgets);
  // Subscribe to the hidden set so toggling visibility re-renders the slot.
  const hidden = useLayoutStore((s) => s.hidden);
  // The running tutorial's current step may want this slot's tab bar to
  // switch to a specific tab (e.g. "Metrics") before its target is measured.
  const tutorial = useTutorialStore((s) => s.activeTutorial());
  const stepIndex = useTutorialStore((s) => s.stepIndex);
  const step = tutorial?.steps[stepIndex];
  const stepForceActive = step?.focusTab && step.focusTab.slot === slot ? step.focusTab.tabId : undefined;

  // A tutorial step may have forced the side rail onto some other tab (e.g.
  // Inspector) to point at it. When the tutorial ends, nothing un-forces
  // that — so without this, finishing a tutorial silently strands the user
  // off the Learn tab, and starting the next tutorial in the curriculum
  // requires manually clicking back. Detect the active→inactive transition
  // here and send the side rail back to Learn once.
  const wasActive = useRef(false);
  const [returnToLearn, setReturnToLearn] = useState(false);
  useEffect(() => {
    if (tutorial) {
      wasActive.current = true;
      // A new tutorial taking over supersedes any pending "return to Learn"
      // from the previous one — otherwise that stale flag would keep forcing
      // Learn on every later step of *this* tutorial that has no focusTab of
      // its own, fighting the step-specific switches and even the user's own
      // manual tab clicks.
      setReturnToLearn(false);
    } else if (wasActive.current) {
      wasActive.current = false;
      if (slot === 'side') setReturnToLearn(true);
    }
  }, [tutorial, slot]);

  const forceActive = stepForceActive ?? (returnToLearn ? 'learn' : undefined);

  const widgets = Object.values(widgetsMap)
    .filter((w) => w.slot === slot && !hidden.has(w.id))
    .sort((a, b) => (a.order ?? 100) - (b.order ?? 100));

  if (widgets.length === 0) return null;

  if (asTabs) {
    const tabs: TabDef[] = widgets.map((w) => {
      const C = w.component;
      return {
        id: w.id,
        label: w.title,
        content: (
          <WidgetBoundary widgetId={w.id} title={w.title}>
            <C />
          </WidgetBoundary>
        ),
      };
    });
    return <Tabs tabs={tabs} forceActive={forceActive} />;
  }

  return (
    <>
      {widgets.map((w) => {
        const C = w.component;
        return (
          <WidgetBoundary key={w.id} widgetId={w.id} title={w.title}>
            <C />
          </WidgetBoundary>
        );
      })}
    </>
  );
}
