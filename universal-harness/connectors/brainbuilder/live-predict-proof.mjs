// Proves the predict-api connector end to end using ONLY universal-harness's
// own layers: connectCdp (GUI layer's transport) to invoke the real Tauri
// IPC command that starts BrainBuilder's predict server, then the harness's
// own introspectApi/executeApi against the real returned URL.
//
// Previously blocked by a real bug in BrainBuilder itself (now fixed, see
// gui/src-tauri/src/main.rs's WEBVIEW2_USER_DATA_FOLDER override): Tauri/wry
// defaults every app to one shared WebView2 profile directory, and since all
// Tauri apps also share the fixed `http://tauri.localhost/` origin, a
// service worker registered by a *different* local Tauri app was
// intercepting BrainBuilder's own page loads, making every IPC command fail
// "not found". Giving BrainBuilder its own profile directory fixed it.
import { connectCdp, introspectApi, executeApi } from '../../dist/src/index.js';
import { writeFile } from 'node:fs/promises';
import path from 'node:path';

// The predict server's cwd is wherever `tauri dev` launched the Rust
// process from (gui/src-tauri), not this script's cwd — pass an absolute
// path so it's unambiguous regardless of how/where the dev server was started.
const datasetPath = path.resolve('../gui/examples/first_run.csv');

const client = await connectCdp('http://localhost:9222');

// Real Tauri v1 IPC: a callback-pair dance through window.__TAURI_IPC__
// (confirmed against the actual window — v1 is what's really installed:
// tauri v1.8.3 in Cargo.lock, @tauri-apps/api@1.6.0 in package-lock.json).
async function invokeIpc(cmd, args = {}) {
  const expr = `
  (function() {
    function transformCallback(callback, once) {
      const identifier = window.crypto.getRandomValues(new Uint32Array(1))[0];
      const prop = '_' + identifier;
      Object.defineProperty(window, prop, {
        value: (result) => { if (once) delete window[prop]; return callback(result); },
        writable: false, configurable: true
      });
      return identifier;
    }
    return new Promise((resolve, reject) => {
      const callback = transformCallback((e) => resolve(e), true);
      const error = transformCallback((e) => reject(e), true);
      window.__TAURI_IPC__({ cmd: ${JSON.stringify(cmd)}, callback, error, ...${JSON.stringify(args)} });
    });
  })()`;
  return client.evaluate(expr);
}

console.log('starting real predict server via Tauri IPC...');
const predictUrl = await invokeIpc('start_predict_server');
console.log('  predict server URL:', predictUrl);

console.log('introspecting the real predict API via universal-harness...');
const manifest = await introspectApi({
  spec: 'connectors/brainbuilder/predict-api.openapi.json',
  baseUrl: predictUrl,
  name: 'BrainBuilder Predict Server',
});
await writeFile('connectors/brainbuilder/predict-api.manifest.json', JSON.stringify(manifest, null, 2));
console.log('  wrote manifest with', manifest.operations.length, 'operation(s)');

const graph = {
  schema_version: 1,
  graph_id: '00000000-0000-0000-0000-000000000001',
  name: 'first-run-example',
  nodes: [
    {
      id: 'n1',
      component: 'scale',
      label: 'Scale',
      hyperparams: {},
      ports: { input_ports: ['x', 'w'], output_ports: ['y'] },
      position: { x: 250, y: 150 },
    },
  ],
  edges: [],
};

console.log('calling POST /predict for real, through universal-harness executeApi...');
const result = await executeApi(manifest, 'predict', {
  body: { graph_json: JSON.stringify(graph), dataset_path: datasetPath, rows: 5 },
});
console.log('  ok:', result.ok);
console.log('  output:', JSON.stringify(result.output));
if (!result.ok) throw new Error('predict call failed: ' + result.error);

console.log('stopping predict server...');
await invokeIpc('stop_predict_server');

client.close();
console.log('done.');
