// Example BrainBuilder widget plugin: a sticky scratch-note pad.
//
// This is a plain ES module loaded at RUNTIME (no build step) by the plugin
// loader. It demonstrates the whole hot-load path: the app hands `register` a
// capability-gated host, and the plugin builds a real React widget using the
// host's shared React instance (`host.react`) — it never imports React itself.
//
// Notes persist in localStorage so the demo survives a reload.
export function register(host) {
  const React = host.react;
  const { useState } = React;
  const KEY = 'brainbuilder.plugin.stickyNote';

  function StickyNote() {
    const [text, setText] = useState(() => localStorage.getItem(KEY) || '');
    const onChange = (e) => {
      setText(e.target.value);
      try {
        localStorage.setItem(KEY, e.target.value);
      } catch (_) {
        /* ignore */
      }
    };
    return React.createElement(
      'div',
      { className: 'bb-panel', style: { padding: 12 } },
      React.createElement('div', { className: 'bb-panel__title', style: { marginBottom: 6 } }, '📝 Sticky Note'),
      React.createElement('textarea', {
        className: 'bb-textarea',
        value: text,
        onChange,
        rows: 6,
        placeholder: 'Jot anything — this whole panel was hot-loaded at runtime.',
      }),
    );
  }

  host.registerWidget({
    id: 'sticky-note',
    title: 'Notes',
    slot: 'side',
    component: StickyNote,
    order: 70,
  });
}
