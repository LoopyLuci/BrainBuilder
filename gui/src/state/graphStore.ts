import { create } from 'zustand';
import {
  Node,
  Edge,
  Connection,
  addEdge,
  NodeChange,
  EdgeChange,
  applyNodeChanges,
  applyEdgeChanges,
} from 'reactflow';
import { v4 as uuidv4 } from 'uuid';
import { ComponentSummary, TrainingConfig } from '../api/tauri';
import { defaultTraining } from '../canvas/utils';

interface GraphSnapshot {
  nodes: Node[];
  edges: Edge[];
}

const HISTORY_LIMIT = 50;

interface GraphState {
  /** Stable id for the current document — generated once (New/first launch)
   * or carried over on Load, never regenerated on every export. This is what
   * lets a checkpoint saved during training be found again by Predict. */
  graphId: string;
  nodes: Node[];
  edges: Edge[];
  selectedNode: string | null;
  /** Component descriptors keyed by component name, fetched once at startup. */
  descriptors: Record<string, ComponentSummary>;
  setDescriptors: (list: ComponentSummary[]) => void;
  setSelectedNode: (id: string | null) => void;
  addNode: (component: string, position: { x: number; y: number }) => void;
  addEdgeToStore: (edge: Edge | Connection) => void;
  onNodesChange: (changes: NodeChange[]) => void;
  onEdgesChange: (changes: EdgeChange[]) => void;
  /** Replace the whole graph (used by New / load / first-run example). Clears
   * undo history. `graphId` defaults to a fresh id (New); pass the loaded
   * graph's id explicitly when restoring a saved document. */
  setGraph: (nodes: Node[], edges: Edge[], graphId?: string) => void;
  updateNodeHyperparams: (id: string, hyperparams: Record<string, unknown>) => void;
  /** Training configuration + selected dataset, edited in the Data/Training panel. */
  training: TrainingConfig;
  setTraining: (t: TrainingConfig) => void;
  setDatasetPath: (path: string) => void;
  /** Undo/redo history of structural edits (add/remove node/edge, hyperparam
   * change). Node-drag position changes are intentionally not checkpointed —
   * they'd flood the stack with one entry per animation frame. */
  past: GraphSnapshot[];
  future: GraphSnapshot[];
  undo: () => void;
  redo: () => void;
}

function checkpoint(state: GraphState): Pick<GraphState, 'past' | 'future'> {
  const snapshot: GraphSnapshot = { nodes: state.nodes, edges: state.edges };
  return { past: [...state.past, snapshot].slice(-HISTORY_LIMIT), future: [] };
}

export const useGraphStore = create<GraphState>((set, get) => ({
  graphId: uuidv4(),
  nodes: [],
  edges: [],
  selectedNode: null,
  descriptors: {},
  past: [],
  future: [],
  setDescriptors: (list) =>
    set({ descriptors: Object.fromEntries(list.map((d) => [d.name, d])) }),
  setSelectedNode: (id) => set({ selectedNode: id }),
  addNode: (component, position) =>
    set((state) => {
      const descriptor = state.descriptors[component];
      // Seed with the descriptor's declared defaults rather than `{}` — a
      // component like `layernorm` or `embedding` needs its hyperparameters
      // (`features`, `vocab_size`, ...) to size its learnable weight
      // correctly (see `resolve_shape_expr` in core/src/runtime/scheduler.rs);
      // an empty hyperparams object silently falls back to a much cruder
      // shape guess instead of erroring, so this default matters for
      // correctness, not just convenience.
      const hyperparams = Object.fromEntries(
        (descriptor?.hyperparameters ?? []).map((h) => [h.name, h.default]),
      );
      return {
        ...checkpoint(state),
        nodes: [
          ...state.nodes,
          {
            id: uuidv4(),
            type: 'default',
            position,
            data: {
              component,
              label: component,
              hyperparams,
              // Snapshot the descriptor onto the node so NodeComponent can
              // render real handles and convertToBBIR can emit real port names.
              descriptor,
            },
          },
        ],
      };
    }),
  addEdgeToStore: (edge) =>
    set((state) => ({
      ...checkpoint(state),
      edges: addEdge(edge, state.edges),
    })),
  onNodesChange: (changes) =>
    set((state) => {
      const structural = changes.some((c) => c.type === 'remove');
      return {
        ...(structural ? checkpoint(state) : {}),
        nodes: applyNodeChanges(changes, state.nodes),
      };
    }),
  onEdgesChange: (changes) =>
    set((state) => {
      const structural = changes.some((c) => c.type === 'remove');
      return {
        ...(structural ? checkpoint(state) : {}),
        edges: applyEdgeChanges(changes, state.edges),
      };
    }),
  setGraph: (nodes, edges, graphId) =>
    set({ nodes, edges, graphId: graphId ?? uuidv4(), selectedNode: null, past: [], future: [] }),
  updateNodeHyperparams: (id, hyperparams) =>
    set((state) => ({
      ...checkpoint(state),
      nodes: state.nodes.map((n) =>
        n.id === id ? { ...n, data: { ...n.data, hyperparams } } : n,
      ),
    })),
  training: defaultTraining(),
  setTraining: (t) => set({ training: t }),
  setDatasetPath: (path) =>
    set((state) => ({
      training: { ...state.training, data_source: { ...state.training.data_source, path_or_uri: path } },
    })),
  undo: () => {
    const state = get();
    const prev = state.past[state.past.length - 1];
    if (!prev) return;
    set({
      nodes: prev.nodes,
      edges: prev.edges,
      past: state.past.slice(0, -1),
      future: [{ nodes: state.nodes, edges: state.edges }, ...state.future],
    });
  },
  redo: () => {
    const state = get();
    const next = state.future[0];
    if (!next) return;
    set({
      nodes: next.nodes,
      edges: next.edges,
      past: [...state.past, { nodes: state.nodes, edges: state.edges }],
      future: state.future.slice(1),
    });
  },
}));
