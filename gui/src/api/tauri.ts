import { invoke } from '@tauri-apps/api/tauri';

export interface BBIRNode {
  id: string;
  component: string;
  label?: string;
  hyperparams: any;
  ports: {
    input_ports: string[];
    output_ports: string[];
  };
  position?: { x: number; y: number };
}

export interface BBIREdge {
  from_node: string;
  from_port: string;
  to_node: string;
  to_port: string;
}

export interface DataSourceConfig {
  source_type: string;
  path_or_uri: string;
  batch_size: number;
  preprocessing: { op: string; params: any }[];
  // Only meaningful when source_type === "text_sequence" — see
  // core/src/data/text.rs. Must match the embedding node's vocab_size
  // hyperparameter (not auto-synced yet).
  sequence_length?: number;
  vocab_size?: number;
  // Only meaningful when source_type === "image_folder" — see core/src/data/vision.rs.
  image_size?: number;
  grayscale?: boolean;
  // Only meaningful when source_type === "text_column" — see core/src/data/tabular_text.rs.
  text_column?: string;
  label_column?: string;
}

export interface TrainingConfig {
  loss: string;
  optimizer: string;
  trainer_type: string;
  hyperparams: any;
  data_source: DataSourceConfig;
}

export interface BBIRGraph {
  graph_id: string;
  name: string;
  nodes: BBIRNode[];
  edges: BBIREdge[];
  training?: TrainingConfig;
}

export interface PortSummary {
  name: string;
  role: 'data' | 'parameter';
  dtype: string;
}

export interface HyperParamSummary {
  name: string;
  param_type: string;
  default: any;
}

export interface ComponentSummary {
  name: string;
  meta_type: string;
  inputs: PortSummary[];
  outputs: PortSummary[];
  hyperparameters: HyperParamSummary[];
}

export async function executeGraph(graph: BBIRGraph): Promise<void> {
  return invoke('execute_graph', { graphJson: JSON.stringify(graph) });
}

// Structural + shape check only (`component::validation::validate_graph`),
// no training — fast feedback before committing to a full run.
export async function validateGraph(graph: BBIRGraph): Promise<void> {
  return invoke('validate_graph', { graphJson: JSON.stringify(graph) });
}

export async function getComponents(): Promise<string[]> {
  return invoke('get_components');
}

// --- Task-first Intent layer (core/src/intent.rs) ---------------------------
// The on-ramp for someone who thinks in outcomes, not graphs: pick a task,
// point at data, get back a validated, trainable model proposal.

export type TaskKind = 'classification' | 'regression';

export interface DataSpec {
  source_type: string; // 'image_folder' | 'text_column' | 'file'
  path: string;
  image_size?: number;
  grayscale?: boolean;
  text_column?: string;
  label_column?: string;
  vocab_size?: number;
}

export interface IntentRequest {
  task: TaskKind;
  data: DataSpec;
}

export interface ProposedModel {
  graph: BBIRGraph;
  class_names: string[];
  feature_count: number;
  num_outputs: number;
  rationale: string;
}

// Inspect the real data and return a validated, ready-to-train model proposal.
// Throws (rejects) with a plain-English message if the data can't support the
// chosen task (e.g. only one class for classification).
export async function proposeModel(request: IntentRequest): Promise<ProposedModel> {
  const json = await invoke<string>('propose_model', { requestJson: JSON.stringify(request) });
  return JSON.parse(json);
}

export interface TransferRequest {
  task: TaskKind;
  data: DataSpec;
  pretrained_file: string;
  pretrained_tensor: string;
}

// Adapt a real pretrained .safetensors backbone (frozen) to the user's data
// with a fresh trainable head. Rejects with a plain-English message if the
// backbone's input dimension doesn't match the data's feature count.
export async function proposeTransferModel(request: TransferRequest): Promise<ProposedModel> {
  const json = await invoke<string>('propose_transfer_model', { requestJson: JSON.stringify(request) });
  return JSON.parse(json);
}

// --- Plain-English diagnostics (core/src/diagnostics.rs) --------------------

export type DiagnosticSeverity = 'error' | 'warning' | 'info';

export interface Diagnostic {
  severity: DiagnosticSeverity;
  title: string;
  explanation: string;
  suggestion: string;
}

// Data-time checks (class imbalance, tiny classes, feature/target leakage) run
// against the real data before training.
export async function diagnoseData(request: IntentRequest): Promise<Diagnostic[]> {
  return invoke('diagnose_data', { requestJson: JSON.stringify(request) });
}

// Training-time check: interpret a loss curve in plain English.
export async function diagnoseTraining(losses: number[]): Promise<Diagnostic[]> {
  return invoke('diagnose_training', { losses });
}

export async function getComponentDescriptors(): Promise<ComponentSummary[]> {
  return invoke('get_component_descriptors');
}

export interface DatasetPreview {
  columns: string[];
  rows: string[][];
}

export async function previewDataset(path: string, limit = 10): Promise<DatasetPreview> {
  return invoke('preview_dataset', { path, limit });
}

export async function saveGraph(path: string, graph: BBIRGraph): Promise<void> {
  return invoke('save_graph', { path, graphJson: JSON.stringify(graph) });
}

export async function loadGraph(path: string): Promise<BBIRGraph> {
  const json = await invoke<string>('load_graph', { path });
  return JSON.parse(json);
}

export interface PredictResult {
  shape: number[];
  values: number[];
}

export async function predict(graph: BBIRGraph, datasetPath: string, rows: number): Promise<PredictResult[]> {
  return invoke('predict', { graphJson: JSON.stringify(graph), datasetPath, rows });
}

export async function hasCheckpoint(graphId: string): Promise<boolean> {
  return invoke('has_checkpoint', { graphId });
}

export async function installComponent(path: string): Promise<void> {
  return invoke('install_component', { path });
}

export interface NervousSystemAuditRecord {
  runtime: string;
  outcome: string;
  duration_ms?: number;
  detail: string;
  logged_at: string;
}

/// Every sandboxed subprocess invocation (Racket/Clojure/Python) the
/// nervous system's `Supervisor`/worker has recorded this session: allowed,
/// capability-denied, or timeout-killed.
export async function getNervousSystemAudit(limit = 100): Promise<NervousSystemAuditRecord[]> {
  return invoke('get_nervous_system_audit', { limit });
}
