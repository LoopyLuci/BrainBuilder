import { invoke } from '@tauri-apps/api/tauri';

export type ModelFormat = 'safe_tensors' | 'gguf' | 'onnx' | 'pytorch_bin';

export interface LocalModel {
  repo_id: string;
  snapshot_path: string;
  formats: ModelFormat[];
  files: string[];
}

export interface OnnxIoSpec {
  name: string;
  dtype: string;
  shape: number[];
}

const CUSTOM_DIRS_KEY = 'brainbuilder.modelHub.customDirs';

// User-configured extra directories to scan alongside the HuggingFace hub
// cache (e.g. "D:\Models\general") — persisted in localStorage so it
// survives an app restart without needing a backend settings file.
export function getCustomModelDirs(): string[] {
  try {
    const raw = localStorage.getItem(CUSTOM_DIRS_KEY);
    return raw ? JSON.parse(raw) : [];
  } catch {
    return [];
  }
}

export function addCustomModelDir(dir: string): string[] {
  const dirs = Array.from(new Set([...getCustomModelDirs(), dir]));
  localStorage.setItem(CUSTOM_DIRS_KEY, JSON.stringify(dirs));
  return dirs;
}

export function removeCustomModelDir(dir: string): string[] {
  const dirs = getCustomModelDirs().filter((d) => d !== dir);
  localStorage.setItem(CUSTOM_DIRS_KEY, JSON.stringify(dirs));
  return dirs;
}

// Real local models already on this machine: the HuggingFace hub cache plus
// any user-configured extra directories.
export async function listLocalModels(): Promise<LocalModel[]> {
  return invoke('list_local_models', { extraDirs: getCustomModelDirs() });
}

// [name, shape, dtype][] — a safetensors file's real tensor manifest, no
// weight data loaded.
export async function inspectSafetensors(path: string): Promise<[string, number[], string][]> {
  return invoke('inspect_safetensors', { path });
}

export async function inspectOnnx(path: string): Promise<[OnnxIoSpec[], OnnxIoSpec[]]> {
  return invoke('inspect_onnx', { path });
}

export async function registerGgufModel(path: string, modelName: string): Promise<void> {
  return invoke('register_gguf_model', { path, modelName });
}

export async function copyModelToBuiltin(src: string, destName?: string): Promise<string> {
  return invoke<string>('copy_model_to_builtin', { src, destName });
}

// LLM-assisted authoring, provider-agnostic. `selector` is a "provider:model"
// string (e.g. "ollama:llama3.2" or "opencode:opencode-go/glm-5.2"); a bare
// model name still defaults to Ollama on the backend. Returns the generated
// graph as a JSON string, matching loadGraph's convention.
export async function generateGraph(description: string, selector: string): Promise<string> {
  return invoke('generate_graph', { description, selector });
}

// --- Multi-provider LLM plumbing (Ollama + OpenCode Go) ---

// [id, display_name][] for the provider selector.
export async function listLlmProviders(): Promise<[string, string][]> {
  return invoke('list_llm_providers');
}

// The models a provider can serve now — live probe for Ollama, published
// roster for OpenCode.
export async function listProviderModels(provider: string): Promise<string[]> {
  return invoke('list_provider_models', { provider });
}

// Store (or clear, with an empty key) a provider's API key in the OS keychain.
export async function setProviderCredentials(provider: string, key: string): Promise<void> {
  return invoke('set_provider_credentials', { provider, key });
}

// Whether a provider's credentials are present (the secret itself never comes
// back to the frontend).
export async function hasProviderCredentials(provider: string): Promise<boolean> {
  return invoke('has_provider_credentials', { provider });
}

// --- Component synthesis (create brand-new components on demand) ---

export interface SmokeReport {
  passed: boolean;
  actual_shape: number[] | null;
  detail: string;
}

export interface SmokeTest {
  input_shapes: number[][];
  expected_shape: number[];
}

export interface SynthesisResult {
  name: string;
  descriptor_edn: string;
  python_code: string;
  smoke_test: SmokeTest;
  smoke: SmokeReport;
  /** How many provider attempts it took (2 means one self-repair round). */
  attempts?: number;
}

// Synthesize a new component from a description. Runs the static gauntlet +
// sandboxed smoke test on the backend; does NOT install. Returns the generated
// descriptor/kernel and the smoke-test result for the user to review.
export async function synthesizeComponent(description: string, selector: string): Promise<SynthesisResult> {
  const json = await invoke<string>('synthesize_component', { description, selector });
  return JSON.parse(json);
}

// Install a synthesized component after review. The backend re-validates and
// re-runs the smoke test, installing (and hot-registering) only on green.
// `result` is what synthesizeComponent returned (its smoke_test shapes are
// reconstructed backend-side from the descriptor).
export async function installSynthesizedComponent(result: SynthesisResult): Promise<void> {
  // Send back the exact artifacts + smoke-test shapes the synthesize call
  // produced; the backend re-parses, re-runs the sandboxed smoke test, and
  // installs only on green.
  const componentJson = JSON.stringify({
    name: result.name,
    descriptor_edn: result.descriptor_edn,
    python_code: result.python_code,
    smoke_test: result.smoke_test,
  });
  return invoke('install_synthesized_component', { componentJson });
}

// --- GPU device picker ---

export interface GpuAdapterInfo {
  name: string;
  backend: string;
  device_type: string;
}

const GPU_PREF_KEY = 'brainbuilder.gpu.preferredAdapter';

export function getPreferredGpu(): string {
  try {
    return localStorage.getItem(GPU_PREF_KEY) ?? '';
  } catch {
    return '';
  }
}

export function setPreferredGpu(name: string): void {
  try {
    if (name) localStorage.setItem(GPU_PREF_KEY, name);
    else localStorage.removeItem(GPU_PREF_KEY);
  } catch {
    /* non-fatal */
  }
}

// Every GPU wgpu can drive on this machine (Vulkan/DX12/Metal/GL).
export async function listGpuAdapters(): Promise<GpuAdapterInfo[]> {
  return invoke('list_gpu_adapters');
}

// Bind a GPU by name (substring, e.g. "7900 XTX") and return the adapter
// actually acquired — the live "does my card work?" probe.
export async function probeGpuAdapter(name: string): Promise<string> {
  return invoke('probe_gpu_adapter', { name });
}

// Tell the backend which GPU native `rust` ops should run on (empty = auto/CPU).
export async function setBackendPreferredGpu(name: string): Promise<void> {
  return invoke('set_preferred_gpu', { name });
}

// Push the persisted GPU preference to the backend. Called once at startup so
// the choice survives a restart and applies before the first graph runs.
// No-op outside the Tauri app (invoke unavailable).
export function syncPreferredGpuToBackend(): void {
  const pref = getPreferredGpu();
  setBackendPreferredGpu(pref).catch(() => {
    /* not in the Tauri runtime, or backend not ready — non-fatal */
  });
}

// --- Auto-tuning ---

export interface TrialResult {
  index: number;
  learning_rate: number | null;
  batch_size: number;
  optimizer: string;
  /** Architecture width scale applied for this trial (1.0 = as authored). */
  width_scale: number;
  score: number | null;
}

// Sweep a small grid of training configs (and, when `searchArch`, a narrower +
// wider model), running each as a real short trial, and return them ranked
// best-first (lowest final loss). `graphJson` is the full BBIR graph.
export async function autotune(graphJson: string, budget: number, searchArch: boolean): Promise<TrialResult[]> {
  return JSON.parse(await invoke<string>('autotune', { graphJson, budget, searchArch }));
}

// --- Luci assistant API ---

export interface LuciMemory {
  id: string;
  memory_type: string;
  content: string;
  embedding?: number[];
  tags: string[];
  confidence: number;
  created_at: string;
  accessed_count: number;
  last_accessed: string;
}

export interface LuciPlan {
  id: string;
  title: string;
  description: string;
  status: string;
  priority: number;
  steps: string[];
  current_step: number;
  created_at: string;
  updated_at: string;
}

export interface LuciReflection {
  id: string;
  session_id: string;
  content: string;
  mood: string;
  insights: string[];
  improvements: string[];
  created_at: string;
}

export interface LuciTool {
  name: string;
  description: string;
  input_schema: Record<string, unknown>;
}

export interface LuciAuditEvent {
  id: string;
  event_type: string;
  source: string;
  details: Record<string, unknown>;
  created_at: string;
}

export interface LuciStatusResponse {
  status: string;
  name: string;
  mood: string;
  energy: number;
  total_interactions: number;
  current_focus: string;
  active_plans: number;
  memory_count: number;
}

export interface LuciChatResponse {
  turn: {
    role: string;
    content: string;
    mood: string;
    timestamp: string;
  };
  plan?: LuciPlan;
}

export async function luciStatus(): Promise<LuciStatusResponse> {
  return invoke('luci_status');
}

export async function luciGreet(): Promise<{ greeting: string; mood: string }> {
  return invoke('luci_greet');
}

export async function luciChat(content: string): Promise<LuciChatResponse> {
  return invoke('luci_chat', { content });
}

export async function luciProposePlan(title: string, description: string, steps: string[]): Promise<LuciPlan> {
  return invoke('luci_propose_plan', { title, description, steps });
}

export async function luciListPlans(): Promise<LuciPlan[]> {
  return invoke('luci_list_plans');
}

export async function luciUpdatePlanStatus(planId: string, status: string): Promise<LuciPlan> {
  return invoke('luci_update_plan_status', { planId, status });
}

export async function luciReflect(content: string): Promise<LuciReflection> {
  return invoke('luci_reflect', { content });
}

export async function luciRecentReflections(limit = 10): Promise<LuciReflection[]> {
  return invoke('luci_recent_reflections', { limit });
}

export async function luciSetPreference(key: string, value: string): Promise<void> {
  return invoke('luci_set_preference', { key, value });
}

export async function luciGetPreference(key: string): Promise<string> {
  return invoke('luci_get_preference', { key });
}

export async function luciRememberFact(content: string, tags: string[] = []): Promise<LuciMemory> {
  return invoke('luci_remember_fact', { content, tags });
}

export async function luciRecallMemories(query: string, limit = 10): Promise<LuciMemory[]> {
  return invoke('luci_recall_memories', { query, limit });
}

export async function luciForgetMemory(id: string): Promise<void> {
  return invoke('luci_forget_memory', { id });
}

export async function luciAudit(eventType: string, source: string, details: Record<string, unknown> = {}): Promise<LuciAuditEvent> {
  return invoke('luci_audit', { eventType, source, details });
}

export async function luciRecentAudit(limit = 20): Promise<LuciAuditEvent[]> {
  return invoke('luci_recent_audit', { limit });
}

export async function luciRegisterTool(tool: LuciTool): Promise<void> {
  return invoke('luci_register_tool', { tool });
}

export async function luciListTools(): Promise<LuciTool[]> {
  return invoke('luci_list_tools');
}

export async function luciImprove(): Promise<{ status: string; changes: string[] }> {
  return invoke('luci_improve');
}

export interface LuciSkill {
  id: string;
  name: string;
  description: string;
  tags: string[];
  params: Record<string, unknown>;
  implementation: string;
  source: string;
  confidence: number;
  success_count: number;
  failure_count: number;
  created_at: string;
  updated_at: string;
}

export interface LuciTaskCase {
  id: string;
  title: string;
  description: string;
  input_example: Record<string, unknown>;
  output_example: Record<string, unknown>;
  tags: string[];
  source: string;
  created_at: string;
}

export interface LuciModelRecord {
  id: string;
  name: string;
  source: string;
  model_type: string;
  format: string;
  path?: string;
  url?: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface LuciDatasetRecord {
  id: string;
  name: string;
  source: string;
  size: number;
  format: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface LuciTrainingJob {
  id: string;
  model_id: string;
  mode: string;
  dataset_ids: string[];
  status: string;
  metrics: Record<string, unknown>;
  artifact_path?: string;
  created_at: string;
  updated_at: string;
}

export async function luciRegisterSkill(skill: LuciSkill): Promise<void> {
  return invoke('luci_register_skill', { skill });
}

export async function luciListSkills(limit = 50): Promise<LuciSkill[]> {
  return invoke('luci_list_skills', { limit });
}

export async function luciObserveAndLearn(task: string, observation: string, outcome: Record<string, unknown>): Promise<string> {
  return invoke('luci_observe_and_learn', { task, observation, outcome });
}

export async function luciImitateSkill(from_case: LuciTaskCase): Promise<LuciSkill> {
  return invoke('luci_imitate_skill', { from_case });
}

export async function luciDecomposeTask(task: string): Promise<LuciPlan> {
  return invoke('luci_decompose_task', { task });
}

export async function luciRegisterModel(model: LuciModelRecord): Promise<void> {
  return invoke('luci_register_model', { model });
}

export async function luciListModels(): Promise<LuciModelRecord[]> {
  return invoke('luci_list_models');
}

export async function luciRegisterDataset(dataset: LuciDatasetRecord): Promise<void> {
  return invoke('luci_register_dataset', { dataset });
}

export async function luciListDatasets(): Promise<LuciDatasetRecord[]> {
  return invoke('luci_list_datasets');
}

export async function luciStartTraining(model_id: string, mode: string, dataset_ids: string[]): Promise<LuciTrainingJob> {
  return invoke('luci_start_training', { model_id, mode, dataset_ids });
}

export async function luciListTrainingJobs(): Promise<LuciTrainingJob[]> {
  return invoke('luci_list_training_jobs');
}

// --- Live Runtime unified frontend bindings ---

export interface LiveRuntimeRunOptions {
  provider?: string;
  model?: string;
  catalogModelId?: string;
  stream?: boolean;
  temperature?: number;
  topP?: number;
  maxTokens?: number;
}

export async function liveRuntimeRun(
  graphJson: string,
  opts: LiveRuntimeRunOptions = {},
): Promise<string> {
  return invoke<string>('live_runtime_run', { graphJson, opts });
}

export async function liveRuntimeStatus(): Promise<string> {
  return invoke<string>('live_runtime_status');
}

export async function liveRuntimeStop(): Promise<string> {
  return invoke<string>('live_runtime_stop');
}

export async function liveRuntimeStreamEvents(): Promise<void> {
  return invoke<void>('live_runtime_stream_events');
}

export async function liveRuntimeSmokeTest(): Promise<string> {
  return invoke<string>('live_runtime_smoke_test');
}
