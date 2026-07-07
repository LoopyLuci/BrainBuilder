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
