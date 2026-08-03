# GUI Panels

Every panel is a widget registered into the [WidgetRegistry](gui-architecture.md#gui-srcwidgets--the-widgetregistry-system)
and rendered by slot. This doc covers what each one does, its main components, and
what backend it talks to. See [GUI Architecture](gui-architecture.md) for the
mechanism that makes all of this pluggable.

## Data panel (`gui/src/data/DataPanel.tsx`)

Actually combines dataset selection *and* the training hyperparameter form in one
panel (`Panel title="Data & Training"`).

- **Dataset picker.** "Choose dataset…" opens a Tauri file dialog filtered to
  `csv`/`parquet`/`txt`. Picking a `.txt` file sets `source_type: 'text_sequence'`
  (word-tokenized, sliding-window sequence loading); anything else is `source_type:
  'file'` (tabular) and triggers a live 8-row preview via `previewDataset`.
- **`text_column` UI.** A checkbox ("This is text data (e.g. reviews)…") toggles
  `source_type` to `'text_column'`, revealing Text column / Label column (optional) /
  Vocab size inputs — the bag-of-words path. This is the only way to attach a
  `text_column` dataset to a manually-built graph; the [Intent panel](#intent-panel)
  builds its own text_column graph automatically instead.
- **Text-sequence controls.** Sequence length and Vocab size inputs when
  `source_type === 'text_sequence'`; a `useEffect` keeps every `embedding` node's
  `vocab_size` hyperparameter in lockstep with the dataset config (working around the
  [manual-coordination gap](core-engine.md#bbir-the-graph-intermediate-representation)
  documented in `bbir.rs`).
- **Preprocessing steps** (file source only): an ordered list of `{op, params}`
  steps (`normalize`/`cast`) applied to named columns every train/predict run, with a
  column-name autocomplete sourced from the preview.
- **Training hyperparameter form.** Driven by a declarative schema rendered via the
  shared [`SchemaForm`](gui-architecture.md#gui-srcwidgets--the-widgetregistry-system)
  widget: loss, optimizer, lr, momentum (sgd only), epochs, batch_size, patience
  (early stop), weight_decay, grad_clip, lr_decay_epochs, label_smoothing, shuffle.
- **Auto-tune button.** Serializes the canvas to BBIR and calls `autotune` — see
  [Autotune](autotune.md#gui) for the full flow, including how the winning
  config/architecture is applied back to the canvas.

## Training panel (`gui/src/training/TrainingDashboard.tsx`)

A pure live viewer — the config lives in the Data panel.

- **Real-time loss chart** — an SVG polyline drawn from `MetricPoint`s received via the
  `metrics-update` Tauri event (see [GUI Architecture's data flow](gui-architecture.md#data-flow-canvas-action--tauri-command--core--back)).
  A new run (step===0 && epoch===0 after prior points exist) resets the curve instead
  of appending a discontinuity.
- Below the chart: current epoch/step/loss, percentage improvement vs. the first
  point, an LR-decay note if `current_lr` changed, and a "stopped early" note if
  applicable. `HelpTip` `?` badges link into the [glossary](glossary.md).
- **Diagnostics.** After ≥3 metric points, debounces 600ms then calls
  `diagnose_training` and renders results via the shared `DiagnosticsList` component —
  see [Diagnostics](core-engine.md#diagnostics-core-srcdiagnosticsrs).

## Predict panel (`gui/src/predict/PredictPanel.tsx`)

Polls `has_checkpoint(graphId)` every 3s to enable itself once training has produced a
checkpoint (there's no "training complete" event yet).

- **Run on first 5 rows** — calls `predict`, showing per-row shape/values.
- **Export checkpoint** — copies a PyTorch checkpoint file to a chosen path via
  `export_checkpoint`.
- **Batch predict** — "Run on entire dataset & export CSV…" calls `batch_predict` (see
  [Batch prediction](core-engine.md#batch-prediction-core-srcbatch_predictrs)) — every
  row, not a preview.
- **Local predict server** — `start_predict_server`/`stop_predict_server`/`predict_server_status`
  start a loopback HTTP `/predict` server; the panel shows the endpoint and a
  ready-to-copy `curl` example. Server state lives in the Rust backend, so it survives
  a page reload.
- **Checkpoint version history** — `list_checkpoint_versions`/`restore_checkpoint_version`.
  Every retrain over an existing checkpoint archives the replaced one; Restore requires
  confirmation and itself archives the checkpoint being replaced first (non-destructive
  both ways).
- **Explain this model (feature importance)** — tabular datasets only. Calls
  `feature_importance`, rendering a ranked list of `column`/`importance` — see
  [Interpretability](core-engine.md#interpretability-core-srcinterpretrs).

## Experiments panel (`gui/src/experiments/ExperimentsPanel.tsx`)

A read-only history viewer. Calls `list_experiments(25)` — every finished training run
is logged automatically by `execute_graph`, so this is the frontend for the
`experiments.sqlite3` history at the repo root. Columns: Model, Architecture, When,
Loss fn, Optimizer, LR, Epochs, and "Loss: first → last" with a computed improvement
percentage.

## Console panel (`gui/src/console/Console.tsx`)

Two tabs:

- **Output** — reads `useLogStore` (capped at 200 entries), the visible sink for
  validation/training/predict errors that every panel writes to via `logInfo`/`logError`
  helpers, replacing invisible `console.error` calls.
- **Nervous System** — polls `get_nervous_system_audit(100)` every 3s, showing every
  sandboxed subprocess invocation (Racket/Clojure via `Supervisor`, Python via the
  persistent worker) with outcome coloring (`allowed`, `capability_denied`,
  `timeout_killed`, `worker_crashed`, `allowed_but_failed`) — see
  [The nervous-system sandbox](runtime-and-devices.md#the-nervous-system-sandbox).
  Reused by the [Agent panel](#llm--synthesis--agent-gui-side-only) to show its own
  sandbox trace filtered to `runtime.includes('agent')`.

## Inspector panel (`gui/src/inspector/Inspector.tsx`)

Shows hyperparameter controls for the selected node, fully descriptor-driven: pulls
`selectedNode.data.descriptor.hyperparameters`, converts to a schema via
`hyperparamsToSchema`, and renders `SchemaForm`. New hyperparameters — including from
synthesized or plugin components — appear automatically with no Inspector code change.

## Component Palette (`gui/src/palette/ComponentPalette.tsx`)

Loads all component descriptors once via `get_component_descriptors`, lists them with
a text filter (matches name or `meta_type`). Each entry is `react-dnd`-draggable onto
the canvas; double-click adds it directly. Tooltip shows data-input count, output
count, and parameter count. See [Component System](component-system.md) for what a
descriptor actually contains.

## Models directory (`gui/src/models/`)

- **`GpuPicker.tsx`** — lists adapters via `list_gpu_adapters`; a select lets the user
  pick a preferred adapter (or Auto), persisted to `localStorage` and pushed to the
  backend via `set_preferred_gpu`. "Test this GPU" calls `probe_gpu_adapter`, showing a
  "bound" chip with the adapter's reported name. See
  [Device propagation](runtime-and-devices.md#device-propagation-gui-pick--training).
- **`ModelHub.tsx`** — scans local model caches (safetensors/GGUF/ONNX/PyTorch-bin) via
  `list_local_models`; custom scan directories persist to `localStorage`. "Inspect"
  calls `inspect_safetensors`/`inspect_onnx` to show tensor names/shapes/dtypes.
  "Register" (GGUF only) calls `register_gguf_model` to make it runnable via `ollama
  run`. Renders `OpenCodeConnect` and `GpuPicker` above itself.
- **`OpenCodeConnect.tsx`** — connects OpenCode Go as a hosted inference provider. A
  password-type input calls `set_provider_credentials('opencode', key)`, which writes
  straight to the OS keychain (never localStorage/app state) — see
  [Credential storage](llm-providers.md#credential-storage-os-keychain).

## Templates directory

The template registry and panel have their own doc — see [Templates](templates.md).

## Tutorial / "Learn" tab (`gui/src/tutorial/`)

- **`curriculum.ts`** defines 47 tutorials, each with a title, blurb, time estimate,
  difficulty, and step list (each step optionally targeting a CSS selector for a
  spotlight and a tab to focus). Covers everything from orientation and building a
  first classifier through distributed training, LoRA fine-tuning, plugins, and GPU
  selection.
- **`tutorialStore.ts`** tracks the active tutorial, step index, and a persisted
  completed-set.
- **`TutorialOverlay.tsx`** renders an SVG scrim with a cut-out spotlight around the
  current step's target element, with a positioned instruction card and
  keyboard navigation.
- **`LearnPanel.tsx`** — the "Learn" tab: every tutorial grouped by difficulty with
  Start/Replay buttons, plus a "Quick glossary" section rendering the same
  [glossary](glossary.md) data used by every in-app `HelpTip`.

## Help system (`gui/src/help/`)

- **`glossary.ts`** — plain-English `{term, short, long}` entries for ML/BrainBuilder
  jargon, written for a zero-prior-knowledge reader. Mirrored in this documentation's
  [Glossary](glossary.md).
- **`HelpTip.tsx`** — a small `?` button that looks up a glossary entry and toggles a
  popover, used inline across nearly every panel.

## Cluster panel (`gui/src/cluster/ClusterConsole.tsx`)

Frontend for the [Cluster & Distributed Training](cluster-and-distributed.md) system.
Polls `get_cluster_status` every 3s. With no cluster yet: enter a display name, then
either "Create My Cluster" (become Manager) or enter a pairing code to join. In a
cluster: shows cluster ID, Manager badge, device list, and (Manager only) "Generate
Pairing Code" (valid 5 minutes). A `DistributedTraining` subcomponent hosts/joins jobs
and reuses the same `metrics-update` event the Training panel listens to — this is real
gradient averaging, not a simulated progress bar.

## Intent panel (`gui/src/intent/IntentPanel.tsx`)

The GUI side of the [Intent layer](core-engine.md#intent-task-first-graph-authoring-core-srcintentrs)
— "Build a Model from a Goal." A user picks a task (classification/regression), a data
source type (image_folder/text_column/file), points at a folder/file, and (optionally)
enables transfer learning. "Build my model" calls `propose_model` or
`propose_transfer_model`, converts the result via `convertFromBBIR`, and drops it onto
the canvas. In parallel, `diagnose_data` surfaces imbalance/leakage warnings via the
shared `DiagnosticsList` component.

## LLM / Synthesis / Agent (GUI side only)

The Rust-side implementations of these three panels each have their own detailed doc —
this section covers only the GUI layer.

- **`llm/ProviderSelector.tsx`** — a shared provider+model select pair, used by every
  LLM-backed panel. See [LLM Providers](llm-providers.md).
- **`llm/LLMAuthor.tsx`** ("Describe a Model") — a description textarea; "Generate"
  calls `generate_graph`, parses the returned BBIR, and places it on the canvas. See
  [Graph authoring pipeline](llm-providers.md#graph-authoring-pipeline-core-srcllmgraph_authorrs).
- **`synthesis/SynthesizePanel.tsx`** ("Synthesize a Component") — a description
  textarea; "Synthesize" calls `synthesize_component`, showing a smoke-test pass/fail
  chip and an "auto-repaired" chip if more than one attempt was needed. "Add to
  canvas"/"Palette only" call `install_synthesized_component`. See
  [Component Synthesis](synthesis.md).
- **`agent/AgentPanel.tsx`** ("Self-Building Agent") — pick an autonomy mode, start a
  session, submit a task, watch streamed `agent-progress` events. "Approve + merge" /
  "Discard" call `agent_approve`/`agent_revert`. See [Self-Building Agent](agent.md).
