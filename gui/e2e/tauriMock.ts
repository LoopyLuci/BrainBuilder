import { Page } from '@playwright/test';

// The GUI has no browser-fallback path for @tauri-apps/api's `invoke()` — it
// always calls `window.__TAURI_IPC__` (see node_modules/@tauri-apps/api/tauri.js),
// which is only injected by the real Tauri webview shell. Under plain
// `vite preview` in Chromium that global does not exist, so calling it throws
// synchronously and every invoke() call rejects. Some call sites already
// handle that gracefully (e.g. `syncPreferredGpuToBackend` in
// src/api/models.ts swallows the rejection), but others (like the component
// palette's `getComponentDescriptors()` on mount) surface an error and leave
// the app without any component descriptors — which blocks template
// application, since `useApplyTemplate` refuses to instantiate a template
// referencing components the descriptor registry doesn't know about.
//
// So for the flows this suite drives, we install a minimal fake
// `window.__TAURI_IPC__` before the app's scripts run, just enough to answer
// `get_component_descriptors` with descriptors for the components the "MLP
// Classifier" template needs (linear, gelu). Every other command resolves to
// a harmless default so nothing hangs. This is a mock at the IPC transport
// boundary, not a re-implementation of Tauri — it does not exercise any real
// Rust code.
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
];

export async function installTauriMock(page: Page): Promise<void> {
  await page.addInitScript((descriptors) => {
    // Mimic the shape of the real IPC bridge closely enough to satisfy
    // @tauri-apps/api's `invoke()` (see node_modules/@tauri-apps/api/tauri.js):
    // it calls `window.__TAURI_IPC__({ cmd, callback, error, ...args })`,
    // where `callback`/`error` are numeric ids registered via
    // `window[\`_${id}\`]` by `transformCallback`.
    (window as any).__TAURI_IPC__ = (message: any) => {
      const respond = (ok: boolean, value: unknown) => {
        const id = ok ? message.callback : message.error;
        const fn = (window as any)[`_${id}`];
        if (typeof fn === 'function') fn(value);
      };
      switch (message.cmd) {
        case 'get_component_descriptors':
          respond(true, descriptors);
          break;
        case 'get_components':
          respond(true, descriptors.map((d: any) => d.name));
          break;
        // Everything else (execute_graph, validate_graph, save_graph, ...)
        // resolves to a harmless default — these flows aren't exercised by
        // this suite, but stubbing them means nothing hangs waiting on a
        // real backend that isn't there.
        default:
          respond(true, null);
      }
    };
  }, COMPONENT_DESCRIPTORS);
}
