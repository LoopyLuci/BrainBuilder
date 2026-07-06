import { useTheme } from '../theme/useTheme';

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
  return (
    <header className="bb-header">
      <div className="bb-header__brand">
        <Logo />
        <span>BrainBuilder</span>
      </div>
      <div className="bb-header__actions">
        <button className="bb-btn bb-btn--ghost bb-btn--sm" onClick={toggle} title="Toggle light/dark theme" aria-label="Toggle theme">
          {theme === 'dark' ? <SunIcon /> : <MoonIcon />}
        </button>
      </div>
    </header>
  );
}
