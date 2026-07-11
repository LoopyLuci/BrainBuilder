// Proves the predict-api connector end to end using ONLY universal-harness's
// own layers: connectCdp (GUI layer's transport) to invoke the real Tauri
// IPC command that starts BrainBuilder's predict server, then the harness's
// own introspectApi/executeApi against the real returned URL.
//
// KNOWN BLOCKER (as of 2026-07-11, unrelated to universal-harness): running
// this against a `tauri dev` session currently fails every IPC command,
// including trivial ones, with "Command X not found". Diagnosis: the
// dev window serves `http://tauri.localhost/` with Tauri v2's JS IPC bridge
// (`window.__TAURI_INTERNALS__`), but `gui/src-tauri/tauri.conf.json`
// (`devPath: http://localhost:5173`) and both `Cargo.toml`/`package.json`
// are pinned to Tauri v1 — a real, pre-existing version/config mismatch in
// BrainBuilder's own Tauri setup, not something this script or
// universal-harness can work around. Fix that mismatch (or run against a
// build where it's already resolved) before this script will get past the
// first `invokeIpc` call.
import { connectCdp, introspectApi, executeApi } from '../../dist/src/index.js';
import { writeFile } from 'node:fs/promises';

// BrainBuilder exposes more than one CDP page target (the main window, plus
// an `assistant.html` secondary window) — urlIncludes picks the main one
// explicitly rather than guessing at whichever the browser lists first.
const client = await connectCdp('http://localhost:9222', { urlIncludes: 'http://tauri.localhost/' });

// This BrainBuilder build uses Tauri v2's IPC entry point
// (window.__TAURI_INTERNALS__.invoke), not the older __TAURI_IPC__ callback
// pattern earlier ad hoc verification scripts in this repo assumed — checked
// directly against the real window rather than guessed.
async function invokeIpc(cmd, args = {}) {
  return client.evaluate(`window.__TAURI_INTERNALS__.invoke(${JSON.stringify(cmd)}, ${JSON.stringify(args)})`);
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
  body: { graph_json: JSON.stringify(graph), dataset_path: 'examples/first_run.csv', rows: 5 },
});
console.log('  ok:', result.ok);
console.log('  output:', JSON.stringify(result.output));
if (!result.ok) throw new Error('predict call failed: ' + result.error);

console.log('stopping predict server...');
await invokeIpc('stop_predict_server');

client.close();
console.log('done.');
