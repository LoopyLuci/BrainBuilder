import { ReactNode, useEffect, useState } from 'react';

export interface TabDef {
  id: string;
  label: string;
  content: ReactNode;
  /** Small dot/count shown next to the label, e.g. for "has content" or an error state. */
  badge?: 'dot' | 'error';
}

export function Tabs({
  tabs,
  defaultTab,
  forceActive,
}: {
  tabs: TabDef[];
  defaultTab?: string;
  /** When set (and changes), switches the active tab — used by the guided
   * tutorial to bring a step's target tab to the front. The user can still
   * click any other tab afterward; this only nudges the initial switch. */
  forceActive?: string;
}) {
  const [active, setActive] = useState(defaultTab ?? tabs[0]?.id);

  useEffect(() => {
    if (forceActive && tabs.some((t) => t.id === forceActive)) setActive(forceActive);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [forceActive]);

  const activeTab = tabs.find((t) => t.id === active) ?? tabs[0];

  return (
    <div className="bb-tabs">
      <div className="bb-tabs__list" role="tablist">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            role="tab"
            data-tutorial={`tab-${tab.id}`}
            aria-selected={tab.id === activeTab?.id}
            className={`bb-tabs__tab ${tab.id === activeTab?.id ? 'bb-tabs__tab--active' : ''}`}
            onClick={() => setActive(tab.id)}
          >
            {tab.label}
            {tab.badge && <span className={`bb-tabs__badge bb-tabs__badge--${tab.badge}`} />}
          </button>
        ))}
      </div>
      <div className="bb-tabs__panel">{activeTab?.content}</div>
    </div>
  );
}
