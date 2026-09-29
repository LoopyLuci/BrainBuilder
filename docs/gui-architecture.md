# GUI Architecture

BrainBuilder's GUI is a Tauri 1.x desktop app: a Rust host process
(`gui/src-tauri/src/main.rs`) embeds a WebView2/WKWebView/webkitgtk shell and exposes a
curated set of async `#[tauri::command]` functions that call into the
`brainbuilder-core` crate. The React 18 + TypeScript frontend (`gui/src/`) talks to
Rust exclusively through `@tauri-apps/api`'s `invoke()`/`listen()`, never touching the
filesystem or subprocesses directly except via Tauri's allowlisted `fs`/`dialog`/`path`/`shell`
plugins.

## `gui/src-tauri/` — the Tauri host

**`AppState`** is the single managed state object: `orchestrator: Mutex<Orchestrator>`
(the [core engine](core-engine.md)), `cluster: Arc<OnceCell<ClusterHandle>>` (filled
asynchronously once the libp2p swarm starts), `observer_url: Arc<OnceCell<String>>`
(the LAN HTTP status page URL), `components_dir: PathBuf`, `agent:
Mutex<Option<AgentSession>>` (the [self-building agent's](agent.md) session), and
`predict_server: Mutex<Option<(String, oneshot::Sender<()>)>>` (the loopback prediction
HTTP server).

### Startup (`main()`)

1. On Windows, sets `WEBVIEW2_USER_DATA_FOLDER` to avoid a shared-profile
   service-worker collision with other Tauri apps — WebView2/wry defaults every app to
   one shared profile directory and origin (`http://tauri.localhost/`), so a service
   worker from a *different* local Tauri app could intercept BrainBuilder's page loads.
   This was a real bug found and fixed during the build of the
   [universal-harness integration](universal-harness.md).
2. Resolves `components_dir` by probing `../components` then `../../components`
   relative to the working directory, since `tauri dev` (cwd = `gui/src-tauri`) and a
   built exe (cwd = `gui/`) differ.
3. Sets `PYTHONPATH` to `components/python` before any Python interaction, so the
   [persistent worker](interop.md#python-worker-componentspython_bb_workerpy) can
   import component modules by name.
4. Constructs the `Orchestrator`, clones its `AppContext` for the Cluster actor, then
   builds the `tauri::Builder` with `.manage(AppState{...})`, a `.setup()` closure
   (metrics event relay + cluster setup), and `.invoke_handler(tauri::generate_handler![...])`.

### Event emission

`setup_metrics_event` subscribes to the core's `subscribe_metrics()` broadcast channel
and, via `tauri::async_runtime::spawn` (plain `tokio::spawn` panics inside `.setup()`
since Tauri's setup callback doesn't run inside an ambient Tokio context), forwards
each `MetricPoint` to the frontend as a `metrics-update` event. `agent_run` similarly
emits `agent-progress` events per phase so the [Agent panel](gui-panels.md#llm--synthesis--agent-gui-side-only)
shows live progress before the (slower) final result returns.

### Tauri commands, grouped by area

| Area | Commands |
|---|---|
| Graph lifecycle | `validate_graph`, `execute_graph`, `save_graph`/`load_graph`, `get_components`/`get_component_descriptors`, `install_component` |
| Prediction/serving | `predict`, `batch_predict`, `start_predict_server`/`stop_predict_server`/`predict_server_status`, `has_checkpoint`, `export_checkpoint`, `list_checkpoint_versions`/`restore_checkpoint_version`, `feature_importance` |
| LLM authoring/providers | `list_llm_providers`, `list_provider_models`, `set_provider_credentials`/`has_provider_credentials`, `generate_graph` — see [LLM Providers](llm-providers.md) |
| Component synthesis | `synthesize_component`, `install_synthesized_component` — see [Component Synthesis](synthesis.md) |
| Self-building agent | `agent_start`/`agent_status`/`agent_run`/`agent_approve`/`agent_revert` — see [Self-Building Agent](agent.md) |
| Auto-tuning | `autotune` — see [Autotune](autotune.md) |
| Intent layer | `propose_model`, `propose_transfer_model`, `diagnose_data`, `diagnose_training` — see [Core Engine](core-engine.md#intent-task-first-graph-authoring-core-srcintentrs) |
| Model hub | `list_local_models`, `inspect_safetensors`, `inspect_onnx`, `register_gguf_model` |
| GPU device picker | `list_gpu_adapters`, `probe_gpu_adapter`, `set_preferred_gpu` — see [Runtime & Devices](runtime-and-devices.md) |
| Cluster/distributed | `get_cluster_status`, `create_cluster`, `generate_pairing_code`, `join_cluster_with_code`, `host_distributed_job`, `list_distributed_jobs`, `join_distributed_job`, `get_distributed_training_status`, `get_observer_url` — see [Cluster & Distributed Training](cluster-and-distributed.md) |
| Observability | `get_nervous_system_audit`, `list_experiments` |

A repeated pattern across several commands (`generate_graph`, `synthesize_component`,
`propose_model`, `propose_transfer_model`): they acquire the component registry's
`std::sync::RwLock` guard only for synchronous read segments and drop it before any
`.await`, since that guard isn't `Send`.

### `Cargo.toml` and `tauri.conf.json`

`gui/src-tauri/Cargo.toml`: `tauri` (features `shell-open`, `dialog-all`,
`fs-create-dir`, `fs-exists`, `path-all`), `brainbuilder-core` (path dependency,
features `gui`, `onnx`, `wgpu`, `pollster`), `tokio` (full), `libp2p` (the Cluster
swarm), `axum` (observer HTTP server), `keyring` (OS secret store). Cargo feature
`custom-protocol` (default) is required for production bundling to serve built
frontend assets via a custom protocol instead of a devserver URL.

`tauri.conf.json`: dev server at `localhost:5173`; the allowlist is deny-by-default
except `shell.open`, `dialog.all`, `path.all`, and `fs` scoped to `createDir`/`exists`
under `$APPDATA` only; single window titled "BrainBuilder", 1280×800, resizable.

## `gui/src/App.tsx` — slot-based rendering

`App.tsx` doesn't hardcode any panel. At module load it calls
`registerBuiltinWidgets()` (registers every first-party panel into the widget
registry), `restorePlugins()` (fire-and-forget re-load of any runtime plugins active at
last shutdown), and `syncPreferredGpuToBackend()`. The component tree is just four
`<SlotRenderer slot="..." />` calls for `palette`, `canvas`, `side` (as tabs), and
`bottom` (as tabs), wrapped in `DndProvider`, plus `TutorialOverlay` and
`ConfirmDialog`. Adding, removing, or replacing a panel — built-in or plugin — never
touches `App.tsx`.

## State stores (`gui/src/state/`)

- **`graphStore.ts`** owns the canvas document: `graphId` (stable across saves,
  matches checkpoints to graphs), reactflow `nodes`/`edges`, `selectedNode`,
  `descriptors` (the component registry snapshot keyed by name), `training`
  (`TrainingConfig`), and a 50-entry undo/redo stack that checkpoints only on
  structural edits (node/edge add/remove, hyperparameter change), not per-drag-frame
  position changes.
- **`providerStore.ts`** holds the app-wide selected LLM provider/model, persisted to
  `localStorage`; exposes `selector()` returning the `"provider:model"` string every
  LLM-backed command expects.
- **`layoutStore.ts`** tracks the set of user-hidden widget ids, persisted separately
  from the widget registry so plugin hot-(un)load churn never clobbers layout intent.
- Also: `ui/confirmStore.ts` (modal confirm dialog state), and feature-local stores
  like `tutorial/tutorialStore.ts` and `console/logStore.ts`.

## Invocation layer (`gui/src/api/`)

Thin, typed wrappers around `invoke()`, one file per feature area: `tauri.ts`
(graph/experiment/prediction/checkpoint/intent/diagnostics commands, plus TS types
mirroring the Rust structs), `cluster.ts`, `models.ts` (model hub, LLM providers,
synthesis, GPU picker, autotune), `metrics.ts` (the `MetricPoint` type used by the
`metrics-update` subscription).

### Dev mock shim (`gui/src/devMock/installDevTauriShim.ts`)

`invoke()` calls `window.__TAURI_IPC__`, which only exists inside the real Tauri
webview. Running `npm run dev` (plain Vite, no Rust backend, for fast UI iteration)
leaves it undefined. `installDevTauriShim()` is a no-op unless `import.meta.env.DEV`
and `window.__TAURI_IPC__` isn't already a function; it then installs a fake IPC bridge
answering a handful of read-only commands with hardcoded placeholder data (five
built-in component descriptors), and rejects any unimplemented command (`execute_graph`,
`predict`, `propose_model`, etc.) with an explicit message telling the developer to use
`npm run tauri -- dev` instead. It never activates in a production build or the real
Tauri shell.

## `gui/src/widgets/` — the WidgetRegistry system

This is the single extension point for all UI, used identically by first-party panels
and runtime-loaded plugins.

- **`types.ts`**: `WidgetSlot` = `'palette' | 'side' | 'bottom' | 'canvas' | 'header'`;
  `WidgetDef` = `{id, title, slot, component, order?, source?}`; `WidgetManifest` =
  `{id, version, entry, description?, capabilities?}` for runtime plugins;
  `PluginCapability` = `'register-widget' | 'read-graph' | 'author-llm'`; `PluginHost`
  is the narrow surface handed to a plugin (`registerWidget`, `react` — the app's own
  React instance, since a hot-loaded module can't `import react`, plus
  capability-gated optional `readGraph`/`authorGraph`).
- **`registry.ts`**: `useWidgetRegistry`, a zustand store of `Record<string, WidgetDef>`.
  `register()` is idempotent by id — re-registering replaces, which is how hot-swap
  works. `bySlot()` filters and sorts by `order`.
- **`SlotRenderer.tsx`**: renders every registered, non-hidden widget for a slot, each
  wrapped in `WidgetBoundary`.
- **`WidgetBoundary.tsx`**: a per-widget React error boundary. A crashing widget
  (including a buggy hot-loaded plugin) renders a contained "hit an error" card with a
  Retry button and logs to the [Console panel](gui-panels.md#console-panel), never
  taking down the shell.
- **`builtins.ts`**: `registerBuiltinWidgets()` registers roughly 16 first-party
  panels (see [GUI Panels](gui-panels.md)) with gapped `order` values so plugins can
  slot between them.
- **`loader.ts` (`usePluginLoader`)**: capability-gated runtime plugin loading.
  `buildHost(manifest)` builds a `PluginHost` scoped strictly to the plugin's declared
  `capabilities` — `registerWidget` throws if `'register-widget'` wasn't declared, and
  namespaces the widget id as `plugin:<id>:<widgetId>` so a plugin can't clobber a
  built-in by id; `readGraph`/`authorGraph` are only attached if declared (the latter
  routes through `generateGraph()` using the app's own provider/model selector — the
  plugin never sees the API key or gets raw `invoke`). `importPluginModule()` fetches
  the plugin source as text and instantiates it via a `Blob` URL + dynamic `import()`
  (bypassing Vite's dev-server import rewriting, and giving a natural point to inspect
  source before running). `load()` validates the manifest, requires an exported
  `register(host)` function, and persists successfully-loaded manifests to
  `localStorage` for `restorePlugins()` to replay at next boot.
- **`PluginsPanel.tsx`**: the loader's UI — panel visibility toggles, three bundled
  example plugins (`sticky-note`, `graph-stats`, `llm-author`, shipped as static JS
  under `gui/public/plugins/`), a manual entry-URL loader, and a list of currently
  loaded plugins with Unload buttons.
- **`SchemaForm.tsx`**: schema-driven form generation. `FieldSchema`/`FormSchema`
  describe `number`/`text`/`boolean`/`select` fields; `hyperparamsToSchema()` bridges a
  [component descriptor's](component-system.md) `hyperparameters` into a `FormSchema`
  so any component's hyperparameter controls — including a freshly
  [synthesized](synthesis.md) one's — render generically, with no per-component
  hand-written form.

## Shell, theme, and shared UI

`shell/Header.tsx` is the top bar: brand/logo plus a light/dark theme toggle driven by
`theme/useTheme.ts`, which reads/writes `localStorage['brainbuilder.theme']` and
sets/removes `data-theme` on the document root (falling back to `prefers-color-scheme`
with no manual choice). `gui/src/ui/` holds shared primitives (`Button`, `Panel`,
`Tabs`, `ConfirmDialog`) and shared class names used throughout widgets.

## The canvas (`gui/src/canvas/`)

Built on `reactflow`. `InfiniteCanvas.tsx` is the `canvas` slot's widget: reads
`nodes`/`edges`/`training` from `graphStore`, renders a `<ReactFlow>` with a single
custom node type (`NodeComponent.tsx`), and enforces constraint-based wiring —
`isValidConnection`/`onConnect` reject self-loops, non-`Data`-role target ports,
already-wired inputs, and dtype mismatches, surfacing a specific reason via the log
store rather than a silent snap-back. Toolbar buttons call `runValidate()` (→
`validateGraph` for a fast structural check) and `exportAndTrain()` (→ `validateGraph`
then `executeGraph`), both first converting reactflow state via `convertToBBIR`.
`useAutosave()` restores the last autosave (or, on first launch,
`gui/examples/first_run.bbir.edn`) once component descriptors are loaded, and saves
every 30s. `CanvasEmptyState.tsx` overlays a "Start from a template" card (see
[Templates](templates.md)) when the graph is empty, disabling templates whose
components aren't in the live registry.

### `convertToBBIR`/`convertFromBBIR` (`gui/src/canvas/utils.ts`)

The GUI↔backend graph converters. `convertToBBIR` maps reactflow `Node`/`Edge` state
plus the stable `graphId` and `TrainingConfig` into a `BBIRGraph` (each node's
`input_ports`/`output_ports` come from its attached component descriptor).
`convertFromBBIR` is the inverse, used on load/autosave-restore, re-attaching each
node's live descriptor from the registry (not persisted in BBIR itself) so
`NodeComponent` can render real handles immediately.

## Data flow: canvas action → Tauri command → core → back

1. User drags a component from the palette or edits hyperparameters →
   `graphStore` mutates `nodes`/`edges` (checkpointing for undo on structural changes).
2. User clicks "Export & Train" → `convertToBBIR` builds a `BBIRGraph`,
   `validate_graph` is called (structural/shape-only, no training), then
   `execute_graph`.
3. Rust's `execute_graph` parses the JSON, spawns a task tapping
   `subscribe_metrics()` to capture first/last loss for the experiment log, and calls
   `orchestrator.execute_graph(graph).await`, which runs the real training loop — see
   [Core Engine](core-engine.md#orchestrator-core-srcorchestratorrs).
4. While training runs, the core broadcasts `MetricPoint`s; the subscriber set up at
   app `.setup()` relays every point to the frontend as a `metrics-update` event.
5. `TrainingDashboard.tsx` listens for `metrics-update` to render the loss chart live —
   no polling.
6. On completion, `execute_graph` logs a record (architecture signature, hyperparameters,
   first/last loss) to `experiment_log`, retrievable via `list_experiments`.
7. Prediction/checkpoint flows follow the same JSON-round-trip convention. Distributed
   training reuses the identical `metrics-update` event path, so hosted/joined
   [Cluster](cluster-and-distributed.md) jobs show progress in the same Metrics tab
   with no separate UI.

## See also

- [GUI Panels](gui-panels.md) — what every panel actually does.
- [Templates](templates.md) — the starter graphs surfaced through the canvas empty
  state and the Templates panel.
- [Universal Harness](universal-harness.md) — how the live GUI is introspected and
  driven as agent-callable operations over CDP, independent of Tauri IPC.
