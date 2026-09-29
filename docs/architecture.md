# Architecture

BrainBuilder is a desktop application for visually building, training, and deploying
neural network graphs. It's split into two halves that communicate over Tauri's IPC:
a Rust orchestration/training engine (`core/`, crate `brainbuilder-core`) and a
React/TypeScript GUI running inside a Tauri desktop shell (`gui/`). A library of
reusable neural-network components (`components/`) is shared between them, described
in a Clojure-flavored data format (EDN) and implemented in Python.

```
┌─────────────────────────────── gui/ (Tauri desktop app) ───────────────────────────────┐
│  React/TypeScript frontend (gui/src)         │  Rust host (gui/src-tauri)               │
│  - reactflow canvas, widget registry,        │  - #[tauri::command] handlers             │
│    zustand stores                            │  - AppState { Orchestrator, Cluster, ... }│
│  - invoke()/listen() over Tauri IPC  ───────────────▶ calls into brainbuilder-core        │
└───────────────────────────────────────────────┴──────────────────────┬────────────────────┘
                                                                          │
┌─────────────────────────────── core/ (brainbuilder-core crate) ────────┴────────────────┐
│  Orchestrator — the central entry point (validate / execute_graph / predict / ...)      │
│    │                                                                                     │
│    ├─ Component system   — ComponentRegistry loads components/*.edn descriptors         │
│    ├─ BBIR                — the graph intermediate representation (nodes/edges/config)  │
│    ├─ Runtime / scheduler — compiles a BBIRGraph into an ExecutionPlan                  │
│    ├─ Interop             — PythonBridge (persistent worker), Racket bridge, Clojure    │
│    │                        bridge — all through a capability-sandboxed subprocess layer │
│    ├─ Data                — DataFusion/Arrow-backed ETL and dataset loading             │
│    ├─ LLM / Intent /      — the self-extending layer: multi-provider LLM access,        │
│    │  Synthesis / Agent /   task-first graph authoring, component synthesis, a          │
│    │  Autotune              self-modifying coding agent, hyperparameter search          │
│    └─ Cluster             — libp2p-based peer-to-peer device mesh + distributed training │
└───────────────────────────────────────────────────────────────────────────────────────┘
         │ subprocess (JSON over stdio + mmap'd tensor files)
         ▼
┌────────────────────── components/python/_bb_worker.py (persistent worker) ─────────────┐
│  Real PyTorch: forward / backward / optimizer.step(), device dispatch (CPU/CUDA/MPS)    │
└──────────────────────────────────────────────────────────────────────────────────────────┘
```

## Guiding design decisions

**Everything is data, not code, where possible.** A neural network graph is BBIR — a
plain JSON/EDN document — not a Rust or Python program. A component is an `.edn`
descriptor plus a small Python file, not a hardcoded type. This is what makes the
self-extending systems ([synthesis](synthesis.md), [the agent](agent.md), the
[GUI's plugin loader](gui-architecture.md#gui-srcwidgets--the-widgetregistry-system))
possible: they can generate, validate, and hot-install new data/code without
recompiling anything.

**PyTorch does the actual math; Rust orchestrates it.** BrainBuilder does not
reimplement autograd or tensor kernels. Real training (forward → loss → backward →
optimizer step) happens inside a single `Python::with_gil`-equivalent session in a
persistent Python worker process, because DLPack tensors round-tripped through Rust
between every op would lose PyTorch's `grad_fn` autograd history. See
[Interop](interop.md).

**Every untrusted or generated execution path is sandboxed.** Racket, Clojure, and the
Python worker all run under a capability-based sandbox ("the nervous system") that
denies filesystem/network access by default and enforces timeouts and (on
Windows/Linux) memory ceilings. Component synthesis and the self-building agent both
route through this same sandbox rather than getting a special exemption. See
[Runtime & Devices](runtime-and-devices.md#the-nervous-system-sandbox) and
[Component Synthesis](synthesis.md).

**The GUI has no hardcoded panels.** `App.tsx` renders purely from a `WidgetRegistry`
by slot; built-in panels and runtime-loaded plugins register into the same registry
through the same capability-gated API. See
[GUI Architecture](gui-architecture.md#gui-srcwidgets--the-widgetregistry-system).

**Nothing pretends to work.** Where a real dependency is missing (a CUDA toolkit, an
`opencode` CLI, a tokenizer library), the code returns a clear, named error rather than
a silent no-op or a fake success. Genuine gaps are documented inline rather than
hidden — see each doc's "known gaps" section.

## Request lifecycle: canvas edit → trained model

1. **Author.** A user drags components onto the `reactflow` canvas (or generates a
   graph from a plain-English goal via the [Intent panel](gui-panels.md#intent-panel),
   or from a description via the [LLM Author panel](gui-panels.md#llm--synthesis--agent-gui-side-only)).
2. **Convert.** `convertToBBIR` (`gui/src/canvas/utils.ts`) turns the reactflow
   `nodes`/`edges` plus the training config into a `BBIRGraph` — see
   [BBIR](core-engine.md#bbir-the-graph-intermediate-representation).
3. **Validate.** The `validate_graph` Tauri command calls
   `Orchestrator::validate`, which structurally checks the graph (every component
   resolves, every port/dtype matches) with no execution — instant feedback in the
   canvas.
4. **Execute.** The `execute_graph` command calls `Orchestrator::execute_graph`, which
   compiles the `BBIRGraph` into an `ExecutionPlan` (the scheduler), loads the dataset
   (`data::source::load_dataset`), and hands both to the trainer, which drives real
   PyTorch training through the [persistent Python worker](interop.md#python-worker-componentspython_bb_workerpy).
5. **Observe.** Training emits `MetricPoint`s on a `tokio::broadcast` channel; the
   Tauri host relays each one to the frontend as a `metrics-update` event, which the
   [Training panel](gui-panels.md#training-panel) renders live with no polling.
6. **Predict / serve.** Once a checkpoint exists, the [Predict panel](gui-panels.md#predict-panel)
   can run inference on a preview batch, export a full-dataset CSV, or start a local
   HTTP `/predict` server.

## Where to go next

- New to the codebase? Read [Core Engine](core-engine.md) next, then
  [Component System](component-system.md).
- Working on the GUI? Read [GUI Architecture](gui-architecture.md) then
  [GUI Panels](gui-panels.md).
- Interested in the self-extending features? Read
  [LLM Providers](llm-providers.md) → [Component Synthesis](synthesis.md) →
  [Self-Building Agent](agent.md) → [Autotune](autotune.md).
- Building or debugging the polyglot bridges? Read [Interop](interop.md).
