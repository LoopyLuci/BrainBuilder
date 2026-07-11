import { Node, Edge } from 'reactflow';
import { v4 as uuidv4 } from 'uuid';
import { ComponentSummary, TrainingConfig } from '../api/tauri';

// Presets: ready-made graphs that lay out every component (with sensible
// settings) so a zero-knowledge user starts from a working architecture and
// customizes, instead of from a blank canvas. A template is declarative — nodes
// reference components by name, edges reference ports by name — and is
// instantiated against the live descriptor registry so each node gets real
// handles + descriptor-seeded defaults, exactly like dragging from the palette.

export interface TemplateNodeSpec {
  /** Local key, unique within the template, used to wire edges. */
  key: string;
  component: string;
  hyperparams?: Record<string, unknown>;
  position: { x: number; y: number };
  label?: string;
}

export interface TemplateEdgeSpec {
  from: string; // source node key
  fromPort: string; // source output port name (= handle id)
  to: string; // target node key
  toPort: string; // target input port name (= handle id)
}

export interface Template {
  id: string;
  name: string;
  category: string;
  description: string;
  nodes: TemplateNodeSpec[];
  edges: TemplateEdgeSpec[];
  /** Optional training settings applied alongside the layout. */
  training?: Partial<TrainingConfig>;
}

export interface InstantiatedTemplate {
  nodes: Node[];
  edges: Edge[];
}

// Turn a declarative template into concrete reactflow nodes/edges, attaching
// the real descriptor to each node (so ports render) and seeding hyperparams
// from the descriptor's defaults, then overriding with the template's values —
// the same node shape `graphStore.addNode` produces.
export function instantiateTemplate(
  template: Template,
  descriptors: Record<string, ComponentSummary>,
): InstantiatedTemplate {
  const keyToId: Record<string, string> = {};

  const nodes: Node[] = template.nodes.map((spec) => {
    const id = uuidv4();
    keyToId[spec.key] = id;
    const descriptor = descriptors[spec.component];
    const seeded = Object.fromEntries((descriptor?.hyperparameters ?? []).map((h) => [h.name, h.default]));
    return {
      id,
      type: 'default',
      position: spec.position,
      data: {
        component: spec.component,
        label: spec.label ?? spec.component,
        hyperparams: { ...seeded, ...(spec.hyperparams ?? {}) },
        descriptor,
      },
    };
  });

  const edges: Edge[] = template.edges.map((e) => ({
    id: uuidv4(),
    source: keyToId[e.from],
    target: keyToId[e.to],
    sourceHandle: e.fromPort,
    targetHandle: e.toPort,
  }));

  return { nodes, edges };
}

// Component names a template references that aren't in the registry — so the
// panel can warn instead of silently placing a broken node.
export function missingComponents(
  template: Template,
  descriptors: Record<string, ComponentSummary>,
): string[] {
  const seen = new Set<string>();
  for (const n of template.nodes) {
    if (!descriptors[n.component] && !seen.has(n.component)) seen.add(n.component);
  }
  return [...seen];
}

export const TEMPLATES: Template[] = [
  {
    id: 'mlp-classifier',
    name: 'MLP Classifier',
    category: 'Starter',
    description: 'A two-layer perceptron (linear → GELU → linear). The simplest thing that trains end-to-end.',
    nodes: [
      { key: 'l1', component: 'linear', hyperparams: { in_features: 4, out_features: 16 }, position: { x: 80, y: 120 } },
      { key: 'act', component: 'gelu', position: { x: 320, y: 120 } },
      { key: 'l2', component: 'linear', hyperparams: { in_features: 16, out_features: 3 }, position: { x: 560, y: 120 } },
    ],
    edges: [
      { from: 'l1', fromPort: 'output', to: 'act', toPort: 'input' },
      { from: 'act', fromPort: 'output', to: 'l2', toPort: 'input' },
    ],
    training: { loss: 'cross_entropy', optimizer: 'adam' },
  },
  {
    id: 'attention-stack',
    name: 'Attention Stack',
    category: 'Transformer',
    description: 'Embedding → multi-head self-attention → layer norm. The core of a transformer, ready to extend.',
    nodes: [
      { key: 'emb', component: 'embedding', hyperparams: { vocab_size: 5000, features: 64 }, position: { x: 80, y: 140 } },
      { key: 'attn', component: 'attention', hyperparams: { num_heads: 4 }, position: { x: 340, y: 140 } },
      { key: 'ln', component: 'layernorm', hyperparams: { features: 64 }, position: { x: 620, y: 140 } },
    ],
    edges: [
      { from: 'emb', fromPort: 'output', to: 'attn', toPort: 'input' },
      { from: 'attn', fromPort: 'output', to: 'ln', toPort: 'input' },
    ],
    training: { loss: 'cross_entropy', optimizer: 'adam' },
  },
  {
    id: 'sentiment-classifier',
    name: 'Text Sentiment Classifier',
    category: 'Text',
    description:
      "A bag-of-words text classifier (linear → GELU → linear) for a spreadsheet of reviews/messages/tickets " +
      'plus a label column — pick your CSV in the Data panel, check "This is text data", and name the text/label ' +
      "columns there. Then set this template's first linear node's in_features to match the real vocabulary " +
      'width the Data panel reports once you train. Proven on a real 60-row positive/negative review example: ' +
      '100% training accuracy — see gui/examples/sentiment.bbir.edn.',
    nodes: [
      { key: 'l1', component: 'linear', hyperparams: { in_features: 200, out_features: 32 }, position: { x: 80, y: 120 } },
      { key: 'act', component: 'gelu', position: { x: 320, y: 120 } },
      { key: 'l2', component: 'linear', hyperparams: { in_features: 32, out_features: 2 }, position: { x: 560, y: 120 } },
    ],
    edges: [
      { from: 'l1', fromPort: 'output', to: 'act', toPort: 'input' },
      { from: 'act', fromPort: 'output', to: 'l2', toPort: 'input' },
    ],
    training: { loss: 'cross_entropy', optimizer: 'adam' },
  },
  {
    id: 'next-word-predictor',
    name: 'Next-Word Predictor',
    category: 'Transformer',
    description:
      'Embedding → real self-attention → select the last position → linear next-word head. Predicts the next ' +
      'word of a text_sequence dataset by attending over the whole context window, not just the previous word ' +
      "(a bare embedding can't disambiguate a shared word like \"the\" that precedes many different next " +
      'words). Proven on a real 4-sentence story cycle: 100% next-word accuracy — see gui/examples/story.bbir.edn.',
    nodes: [
      { key: 'embed', component: 'embedding', hyperparams: { vocab_size: 60, embedding_dim: 32 }, position: { x: 60, y: 120 } },
      { key: 'attn', component: 'attention', hyperparams: { features: 32, num_heads: 4 }, position: { x: 300, y: 120 } },
      { key: 'pool', component: 'select_last', position: { x: 560, y: 120 } },
      { key: 'proj', component: 'linear', hyperparams: { in_features: 32, out_features: 60 }, position: { x: 780, y: 120 } },
    ],
    edges: [
      { from: 'embed', fromPort: 'output', to: 'attn', toPort: 'input' },
      { from: 'attn', fromPort: 'output', to: 'pool', toPort: 'input' },
      { from: 'pool', fromPort: 'output', to: 'proj', toPort: 'input' },
    ],
    training: { loss: 'cross_entropy', optimizer: 'adam' },
  },
  {
    id: 'dspark-drafter',
    name: 'DSpark Speculative Drafter',
    category: 'Speculative decoding',
    description:
      'The DeepSeek-style drafter head stack: a parallel intern drafts a block of tokens, a low-rank Markov head cures suffix decay, and a confidence head gates early termination. Laid out with matched dimensions — wire it onto your own backbone.',
    nodes: [
      { key: 'intern', component: 'parallel_intern', hyperparams: { features: 256, vocab: 1000, draft_len: 8 }, position: { x: 120, y: 60 } },
      { key: 'markov', component: 'low_rank_markov_head', hyperparams: { features: 256, rank: 32, vocab: 1000 }, position: { x: 120, y: 220 } },
      { key: 'conf', component: 'confidence_head', hyperparams: { features: 256, proj: 128 }, position: { x: 120, y: 360 } },
    ],
    // The three heads read the same backbone hidden state; the user wires that
    // source in. No inter-head edges — they are parallel drafting heads.
    edges: [],
  },
];
