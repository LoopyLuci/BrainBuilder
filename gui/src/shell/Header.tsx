import { useTheme } from '../theme/useTheme';
import { useLayoutStore } from '../state/layoutStore';

function SunIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
      <circle cx="12" cy="12" r="4" />
      <path d="M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4" />
    </svg>
  );
}

function MoonIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M20 14.5A8.5 8.5 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5Z" />
    </svg>
  );
}

function PaletteIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2.69l5.66 5.66a8 8 0 1 1-11.31 0z" />
    </svg>
  );
}

function RightIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 7h16M4 12h10M4 17h6" />
    </svg>
  );
}

function BottomIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M4 7h16M4 12h16M4 17h16" />
    </svg>
  );
}

function LuciIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
      <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
    </svg>
  );
}

function Logo() {
  return (
    <svg width="18" height="18" viewBox="0 0 24 24" fill="none">
      <circle cx="6" cy="6" r="3" fill="var(--accent)" />
      <circle cx="18" cy="6" r="3" fill="var(--accent)" opacity="0.6" />
      <circle cx="12" cy="18" r="3" fill="var(--accent)" opacity="0.35" />
      <path d="M8.5 7.5L15.5 7.5M7.5 8.7L11 16M16.5 8.7L13 16" stroke="var(--accent)" strokeWidth="1.4" strokeLinecap="round" opacity="0.5" />
    </svg>
  );
}

export function Header() {
  const { theme, toggle } = useTheme();
  const paletteVisible = useLayoutStore((s) => s.paletteVisible);
  const sideRailVisible = useLayoutStore((s) => s.sideRailVisible);
  const rightRailVisible = useLayoutStore((s) => s.rightRailVisible);
  const bottomHeight = useLayoutStore((s) => s.bottomHeight);
  const togglePalette = useLayoutStore((s) => s.togglePalette);
  const toggleSideRail = useLayoutStore((s) => s.toggleSideRail);
  const toggleRightRail = useLayoutStore((s) => s.toggleRightRail);
  const setBottomHeight = useLayoutStore((s) => s.setBottomHeight);

  return (
    <header className="bb-header">
      <div className="bb-header__brand">
        <Logo />
        <span>BrainBuilder</span>
      </div>
      <div className="bb-header__actions">
        <button
          className="bb-btn bb-btn--ghost bb-btn--sm"
          onClick={togglePalette}
          title={paletteVisible ? 'Hide palette' : 'Show palette'}
          aria-label={paletteVisible ? 'Hide palette' : 'Show palette'}
        >
          <PaletteIcon />
        </button>
        <button
          className="bb-btn bb-btn--ghost bb-btn--sm"
          onClick={toggleSideRail}
          title={sideRailVisible ? 'Close Luci Agent Dashboard' : 'Open Luci Agent Dashboard'}
          aria-label={sideRailVisible ? 'Close Luci Agent Dashboard' : 'Open Luci Agent Dashboard'}
        >
          <LuciIcon />
        </button>
        <button
          className="bb-btn bb-btn--ghost bb-btn--sm"
          onClick={toggleRightRail}
          title={rightRailVisible ? 'Hide right panel' : 'Show right panel'}
          aria-label={rightRailVisible ? 'Hide right panel' : 'Show right panel'}
        >
          <RightIcon />
        </button>
        <button
          className="bb-btn bb-btn--ghost bb-btn--sm"
          onClick={() => setBottomHeight(bottomHeight === 0 ? 260 : 0)}
          title={bottomHeight === 0 ? 'Show bottom panel' : 'Hide bottom panel'}
          aria-label={bottomHeight === 0 ? 'Show bottom panel' : 'Hide bottom panel'}
        >
          <BottomIcon />
        </button>
        <button
          className="bb-btn bb-btn--ghost bb-btn--sm"
          onClick={toggle}
          title="Toggle light/dark theme"
          aria-label="Toggle theme"
        >
          {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
        </button>
      </div>
    </header>
  );
}
