import { describe, it, expect } from 'vitest';
import { TEMPLATES, instantiateTemplate, missingComponents, Template } from './registry';
import type { ComponentSummary } from '../api/tauri';

// A minimal descriptor stand-in for the components the templates reference.
function descriptorsFor(names: string[]): Record<string, ComponentSummary> {
  return Object.fromEntries(
    names.map((n) => [n, { name: n, meta_type: 'stateful-module', hyperparameters: [], inputs: [], outputs: [] } as unknown as ComponentSummary]),
  );
}

describe('template registry', () => {
  it('every template edge references a node key that exists', () => {
    for (const t of TEMPLATES) {
      const keys = new Set(t.nodes.map((n) => n.key));
      for (const e of t.edges) {
        expect(keys.has(e.from), `${t.id}: edge from ${e.from}`).toBe(true);
        expect(keys.has(e.to), `${t.id}: edge to ${e.to}`).toBe(true);
      }
    }
  });

  it('instantiate wires edges to the generated node ids and applies hyperparams', () => {
    const mlp = TEMPLATES.find((t) => t.id === 'mlp-classifier') as Template;
    const descriptors = descriptorsFor(mlp.nodes.map((n) => n.component));
    const { nodes, edges } = instantiateTemplate(mlp, descriptors);
    const ids = new Set(nodes.map((n) => n.id));
    expect(nodes).toHaveLength(mlp.nodes.length);
    expect(edges).toHaveLength(mlp.edges.length);
    for (const e of edges) {
      expect(ids.has(e.source!)).toBe(true);
      expect(ids.has(e.target!)).toBe(true);
    }
    // Template hyperparams land on the node.
    const first = nodes[0];
    expect(first.data.hyperparams.in_features).toBe(4);
    expect(first.data.hyperparams.out_features).toBe(16);
  });

  it('missingComponents flags components not in the registry', () => {
    const dspark = TEMPLATES.find((t) => t.id === 'dspark-drafter') as Template;
    expect(missingComponents(dspark, {})).toEqual(
      expect.arrayContaining(['parallel_intern', 'low_rank_markov_head', 'confidence_head']),
    );
    expect(missingComponents(dspark, descriptorsFor(dspark.nodes.map((n) => n.component)))).toEqual([]);
  });
});
