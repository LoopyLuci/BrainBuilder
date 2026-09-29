import { Node, Edge } from 'reactflow';
import { BBIRGraph, ComponentSummary, TrainingConfig } from '../api/tauri';

export const defaultTraining = (): TrainingConfig => ({
  loss: 'mse',
  optimizer: 'sgd',
  trainer_type: 'standard',
  hyperparams: { lr: 0.01 },
  data_source: {
    source_type: 'file',
    path_or_uri: '',
    batch_size: 32,
    preprocessing: [],
  },
});

export function convertToBBIR(
  nodes: Node[],
  edges: Edge[],
  graphId: string,
  name = 'untitled',
  training: TrainingConfig = defaultTraining(),
): BBIRGraph {
  return {
    // Stable across calls (caller owns it, from graphStore) — this id is how
    // a trained checkpoint gets matched back up with the graph in Predict.
    // Generating a fresh random id per conversion (the original approach)
    // would silently orphan every checkpoint the moment training finished.
    graph_id: graphId,
    name,
    nodes: nodes.map((n) => {
      const descriptor: ComponentSummary | undefined = n.data.descriptor;
      // input_ports lists *all* inputs (data + parameter): the backend passes
      // them to the component and classifies params from the descriptor. If a
      // node has no descriptor (shouldn't happen for palette-added nodes), fall
      // back to generic single-port naming.
      const input_ports = descriptor
        ? descriptor.inputs.map((p) => p.name)
        : ['input'];
      const output_ports = descriptor
        ? descriptor.outputs.map((p) => p.name)
        : ['output'];
      return {
        id: n.id,
        component: n.data.component,
        label: n.data.label,
        hyperparams: n.data.hyperparams || {},
        ports: { input_ports, output_ports },
        position: n.position,
      };
    }),
    edges: edges.map((e) => ({
      from_node: e.source,
      // ReactFlow carries the connected handle ids (= real port names) when the
      // node renders multiple handles; fall back to first out/in if absent.
      from_port: e.sourceHandle || firstOutput(nodes, e.source),
      to_node: e.target,
      to_port: e.targetHandle || firstDataInput(nodes, e.target),
    })),
    training,
  };
}

/** Inverse of convertToBBIR — rebuilds ReactFlow nodes/edges from a loaded
 * BBIRGraph, re-attaching each node's live descriptor (not persisted in BBIR
 * itself, since it's derived data already available from the component
 * registry) so NodeComponent can render real handles immediately. */
export function convertFromBBIR(
  graph: BBIRGraph,
  descriptors: Record<string, ComponentSummary>,
): { nodes: Node[]; edges: Edge[] } {
  const nodes: Node[] = graph.nodes.map((n, i) => ({
    id: n.id,
    type: 'default',
    position: n.position ?? { x: 100 + (i % 5) * 160, y: 100 + Math.floor(i / 5) * 120 },
    data: {
      component: n.component,
      label: n.label ?? n.component,
      hyperparams: n.hyperparams ?? {},
      descriptor: descriptors[n.component],
    },
  }));
  const edges: Edge[] = graph.edges.map((e, i) => ({
    id: `e-${e.from_node}-${e.to_node}-${i}`,
    source: e.from_node,
    sourceHandle: e.from_port,
    target: e.to_node,
    targetHandle: e.to_port,
  }));
  return { nodes, edges };
}

function firstOutput(nodes: Node[], id: string): string {
  const d: ComponentSummary | undefined = nodes.find((n) => n.id === id)?.data.descriptor;
  return d?.outputs[0]?.name ?? 'output';
}

function firstDataInput(nodes: Node[], id: string): string {
  const d: ComponentSummary | undefined = nodes.find((n) => n.id === id)?.data.descriptor;
  return d?.inputs.find((p) => p.role === 'data')?.name ?? 'input';
}
