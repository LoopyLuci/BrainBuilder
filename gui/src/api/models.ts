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

// Local-first LLM-assisted authoring (Ollama). Returns the generated graph
// as a JSON string, matching loadGraph's convention.
export async function generateGraph(description: string, model: string): Promise<string> {
  return invoke('generate_graph', { description, model });
}
