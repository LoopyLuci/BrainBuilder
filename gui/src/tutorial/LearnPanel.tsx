import { Panel } from '../ui/Panel';
import { Button } from '../ui/Button';
import { CURRICULUM, Difficulty } from './curriculum';
import { useTutorialStore } from './tutorialStore';
import { GLOSSARY } from '../help/glossary';

const DIFFICULTY_LABEL: Record<Difficulty, string> = {
  beginner: 'Beginner',
  intermediate: 'Intermediate',
  advanced: 'Advanced',
};

const DIFFICULTY_ORDER: Difficulty[] = ['beginner', 'intermediate', 'advanced'];

// The built-in manual: a guided tutorial list (simple -> complex, exactly the
// curriculum a total beginner — including a child — needs to go from "what is
// this app" to building real models) plus a plain-English glossary. Both pull
// from the same curriculum/glossary data other parts of the app use (tutorial
// steps, HelpTips), so the "manual" is never out of sync with what's taught
// live.
export function LearnPanel() {
  const completed = useTutorialStore((s) => s.completed);
  const start = useTutorialStore((s) => s.start);

  return (
    <Panel title="Learn">
      <p className="bb-text-muted" style={{ margin: 0 }}>
        New here? Start at the top and work down — each tutorial builds on the last, from your first click to
        building a whole model by hand.
      </p>

      {DIFFICULTY_ORDER.map((level) => {
        const tutorials = CURRICULUM.filter((t) => t.difficulty === level);
        if (tutorials.length === 0) return null;
        return (
          <div key={level} style={{ marginTop: 10 }}>
            <label className="bb-label">{DIFFICULTY_LABEL[level]}</label>
            {tutorials.map((t) => {
              const done = completed.has(t.id);
              return (
                <div key={t.id} className="bb-card" style={{ marginBottom: 8, display: 'flex', flexDirection: 'column', gap: 4 }}>
                  <div style={{ display: 'flex', alignItems: 'center', gap: 6 }}>
                    <strong>{t.title}</strong>
                    {done && <span className="bb-text-success" title="Completed">✓</span>}
                  </div>
                  <p className="bb-text-muted" style={{ margin: 0 }}>
                    {t.blurb} · ~{t.minutes} min
                  </p>
                  <Button variant={done ? 'secondary' : 'primary'} onClick={() => start(t.id)}>
                    {done ? 'Replay' : 'Start'}
                  </Button>
                </div>
              );
            })}
          </div>
        );
      })}

      <div style={{ marginTop: 16 }}>
        <label className="bb-label">Quick glossary</label>
        <p className="bb-text-muted" style={{ margin: '0 0 6px' }}>
          Jargon, explained in plain English. You'll also see a little "?" next to these words around the app.
        </p>
        <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
          {Object.values(GLOSSARY).map((entry) => (
            <div key={entry.term}>
              <strong>{entry.term}</strong>
              <p className="bb-text-muted" style={{ margin: 0 }}>{entry.short}</p>
            </div>
          ))}
        </div>
      </div>
    </Panel>
  );
}
