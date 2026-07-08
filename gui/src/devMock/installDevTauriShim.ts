// The GUI's `invoke()` (see node_modules/@tauri-apps/api/tauri.js) calls
// `window.__TAURI_IPC__` directly — that global only exists inside the real
// Tauri webview shell. Launching the plain Vite dev server (`npm run dev`,
// used for fast UI iteration / browser preview without the Rust backend)
// leaves it undefined, so every `invoke()` call throws synchronously and
// surfaces as e.g. "Loading components failed: TypeError: window.__TAURI_IPC__
// is not a function".
//
// In dev mode only, and only when no real IPC bridge is present, install a
// minimal fake bridge so the UI has something to render instead of erroring
// on load. This never runs in a production build or inside the real Tauri
// shell (`tauri dev`/the packaged app) — there `window.__TAURI_IPC__` is
// already defined before this module runs, so `install()` is a no-op.
const COMPONENT_DESCRIPTORS = [
  {
    name: 'linear',
    meta_type: 'layer',
    inputs: [{ name: 'input', role: 'data', dtype: 'f32' }],
    outputs: [{ name: 'output', role: 'data', dtype: 'f32' }],
    hyperparameters: [
      { name: 'in_features', param_type: 'int', default: 4 },
      { name: 'out_features', param_type: 'int', default: 16 },
    ],
  },
  {
    name: 'gelu',
    meta_type: 'activation',
    inputs: [{ name: 'input', role: 'data', dtype: 'f32' }],
    outputs: [{ name: 'output', role: 'data', dtype: 'f32' }],
    hyperparameters: [],
  },
  {
    name: 'relu',
    meta_type: 'activation',
    inputs: [{ name: 'input', role: 'data', dtype: 'f32' }],
    outputs: [{ name: 'output', role: 'data', dtype: 'f32' }],
    hyperparameters: [],
  },
  {
    name: 'conv2d',
    meta_type: 'layer',
    inputs: [{ name: 'input', role: 'data', dtype: 'f32' }],
    outputs: [{ name: 'output', role: 'data', dtype: 'f32' }],
    hyperparameters: [
      { name: 'out_channels', param_type: 'int', default: 8 },
      { name: 'kernel_size', param_type: 'int', default: 3 },
    ],
  },
  {
    name: 'dropout',
    meta_type: 'regularization',
    inputs: [{ name: 'input', role: 'data', dtype: 'f32' }],
    outputs: [{ name: 'output', role: 'data', dtype: 'f32' }],
    hyperparameters: [{ name: 'p', param_type: 'float', default: 0.1 }],
  },
];

function respond(message: any, ok: boolean, value: unknown) {
  const id = ok ? message.callback : message.error;
  const fn = (window as any)[`_${id}`];
  if (typeof fn === 'function') fn(value);
}

export function installDevTauriShim(): void {
  if (!import.meta.env.DEV) return;
  if (typeof window === 'undefined' || typeof (window as any).__TAURI_IPC__ === 'function') return;

  console.info(
    '[BrainBuilder] Running without the Tauri backend (plain browser preview) — using a stubbed IPC bridge. ' +
      'Data/training/prediction commands will return placeholder results. Use `npm run tauri -- dev` for the real app.',
  );

  (window as any).__TAURI_IPC__ = (message: any) => {
    switch (message.cmd) {
      case 'get_component_descriptors':
        respond(message, true, COMPONENT_DESCRIPTORS);
        return;
      case 'get_components':
        respond(message, true, COMPONENT_DESCRIPTORS.map((d) => d.name));
        return;
      case 'diagnose_data':
      case 'diagnose_training':
        respond(message, true, []);
        return;
      case 'has_checkpoint':
        respond(message, true, false);
        return;
      case 'list_llm_providers':
      case 'list_gpu_adapters':
      case 'list_local_models':
      case 'list_distributed_jobs':
        respond(message, true, []);
        return;
      case 'has_provider_credentials':
        respond(message, true, false);
        return;
      case 'get_nervous_system_audit':
        respond(message, true, []);
        return;
      default:
        // Unimplemented in the dev shim (e.g. propose_model, execute_graph,
        // predict) — reject with a clear, non-crashing message rather than
        // hanging or pretending to succeed with fabricated training results.
        respond(message, false, `"${message.cmd}" needs the real BrainBuilder backend — run the app via Tauri, not the plain dev server.`);
    }
  };
}
