import { useWidgetRegistry } from './registry';
import { WidgetBoundary } from './WidgetBoundary';
import { WidgetSlot } from './types';
import { Tabs, TabDef } from '../ui/Tabs';

// Renders every widget registered for a slot, each inside its own error
// boundary. `side`/`bottom` render as tabs (matching the previous shell);
// `palette`/`canvas`/`header` render their widgets stacked, since those slots
// are single-purpose today but stay pluggable.
export function SlotRenderer({ slot, asTabs }: { slot: WidgetSlot; asTabs?: boolean }) {
  // Subscribe to the widgets map so runtime (un)registration re-renders.
  const widgetsMap = useWidgetRegistry((s) => s.widgets);
  const widgets = Object.values(widgetsMap)
    .filter((w) => w.slot === slot)
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
    return <Tabs tabs={tabs} />;
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
