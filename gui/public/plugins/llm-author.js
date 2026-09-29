// Example BrainBuilder widget plugin: a mini LLM graph author.
//
// Demonstrates the 'author-llm' capability: the plugin asks the app's currently
// selected provider/model to author a graph from a description via
// `host.authorGraph`. The API key + provider selection stay entirely app-side —
// the plugin never sees credentials and never gets a raw `invoke`, mirroring
// the backend's deny-by-default capability model. `authorGraph` is only present
// because the manifest declared 'author-llm'.
export function register(host) {
  const React = host.react;
  const { useState } = React;

  function MiniAuthor() {
    const [desc, setDesc] = useState('');
    const [out, setOut] = useState('');
    const [busy, setBusy] = useState(false);

    const run = async () => {
      if (!desc.trim() || !host.authorGraph) return;
      setBusy(true);
      setOut('');
      try {
        const graph = await host.authorGraph(desc);
        setOut(typeof graph === 'string' ? graph : JSON.stringify(graph, null, 2));
      } catch (e) {
        setOut('Error: ' + e);
      } finally {
        setBusy(false);
      }
    };

    return React.createElement(
      'div',
      { className: 'bb-panel', style: { padding: 12 } },
      React.createElement('div', { className: 'bb-panel__title', style: { marginBottom: 6 } }, '✨ Mini Author (plugin)'),
      React.createElement('textarea', {
        className: 'bb-textarea',
        value: desc,
        onChange: (e) => setDesc(e.target.value),
        rows: 2,
        placeholder: host.authorGraph
          ? 'Describe a model — authored via the app’s selected LLM.'
          : "This plugin needs the 'author-llm' capability.",
      }),
      React.createElement(
        'button',
        { className: 'bb-tabs__tab', style: { marginTop: 6 }, onClick: run, disabled: busy || !host.authorGraph },
        busy ? 'Authoring…' : 'Author graph',
      ),
      out
        ? React.createElement(
            'pre',
            {
              style: {
                marginTop: 6,
                maxHeight: 140,
                overflow: 'auto',
                fontFamily: 'var(--font-mono)',
                fontSize: 11,
                whiteSpace: 'pre-wrap',
              },
            },
            out,
          )
        : null,
    );
  }

  host.registerWidget({
    id: 'mini-author',
    title: 'Author+',
    slot: 'side',
    component: MiniAuthor,
    order: 80,
  });
}
