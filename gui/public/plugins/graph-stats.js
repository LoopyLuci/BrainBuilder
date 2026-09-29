// Example BrainBuilder widget plugin: live graph statistics.
//
// Demonstrates a plugin using a DECLARED capability beyond registering a
// widget: it reads the current graph through `host.readGraph` (only present
// because the manifest declared the 'read-graph' capability — the loader omits
// it otherwise). A capability the plugin didn't ask for simply isn't on the
// host object, mirroring the backend's deny-by-default model.
export function register(host) {
  const React = host.react;
  const { useState } = React;

  function GraphStats() {
    const [, forceTick] = useState(0);

    // readGraph returns a snapshot; a refresh button re-reads it. (A production
    // plugin could subscribe, but the point here is the capability boundary.)
    const graph = host.readGraph ? host.readGraph() : { nodes: [], edges: [] };
    const components = {};
    for (const n of graph.nodes) {
      const c = (n && n.data && n.data.component) || 'unknown';
      components[c] = (components[c] || 0) + 1;
    }
    const rows = Object.entries(components).sort((a, b) => b[1] - a[1]);

    return React.createElement(
      'div',
      { className: 'bb-panel', style: { padding: 12 } },
      React.createElement(
        'div',
        { style: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: 6 } },
        React.createElement('div', { className: 'bb-panel__title' }, '📊 Graph Stats'),
        React.createElement(
          'button',
          { className: 'bb-tabs__tab', onClick: () => forceTick((t) => t + 1) },
          'Refresh',
        ),
      ),
      React.createElement(
        'div',
        { className: 'bb-text-muted', style: { fontSize: 12 } },
        `${graph.nodes.length} node(s), ${graph.edges.length} edge(s)`,
      ),
      rows.length === 0
        ? React.createElement('div', { className: 'bb-empty' }, 'Canvas is empty.')
        : React.createElement(
            'ul',
            { className: 'bb-list', style: { marginTop: 6 } },
            rows.map(([name, count]) =>
              React.createElement(
                'li',
                { key: name, className: 'bb-list-item bb-text-muted', style: { fontSize: 11 } },
                `${name} × ${count}`,
              ),
            ),
          ),
    );
  }

  host.registerWidget({
    id: 'graph-stats',
    title: 'Stats',
    slot: 'bottom',
    component: GraphStats,
    order: 50,
  });
}
