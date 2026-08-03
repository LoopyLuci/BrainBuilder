export type NodeType =
  | "model"
  | "adapter"
  | "router"
  | "data"
  | "prompt"
  | "output"
  | "custom";

export interface OmniNode {
  id: string;
  type: NodeType;
  label: string;
  modelId?: string;
  position: { x: number; y: number };
  config?: Record<string, unknown>;
  data?: Record<string, unknown>;
}

export interface OmniEdge {
  id: string;
  source: string;
  target: string;
  sourceHandle?: string;
  targetHandle?: string;
}

export interface CanvasSnapshot {
  nodes: OmniNode[];
  edges: OmniEdge[];
}
