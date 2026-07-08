import { useState } from 'react';
import { GLOSSARY } from './glossary';

// A small "?" badge that explains a term in plain English on click — sprinkled
// next to jargon (hyperparameters, task types, ML terms) throughout the app so
// a first-time user never hits an unexplained word. Keyboard-accessible (a
// real <button>, not a hover-only div) since a click/tap is more discoverable
// for a new user than a hover, and works on touch devices too.
export function HelpTip({ term }: { term: string }) {
  const [open, setOpen] = useState(false);
  const entry = GLOSSARY[term];
  if (!entry) return null;

  return (
    <span className="bb-helptip">
      <button
        type="button"
        className="bb-helptip__trigger"
        aria-label={`What is ${entry.term}?`}
        aria-expanded={open}
        onClick={() => setOpen((o) => !o)}
      >
        ?
      </button>
      {open && (
        <div className="bb-helptip__popover" role="tooltip">
          <strong>{entry.term}</strong>
          <p>{entry.long}</p>
          <button type="button" className="bb-helptip__close" onClick={() => setOpen(false)}>
            Got it
          </button>
        </div>
      )}
    </span>
  );
}
