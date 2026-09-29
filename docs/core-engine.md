# Core Engine

The Rust crate `brainbuilder-core` (`core/`) is BrainBuilder's orchestration and
training engine. This doc covers everything under `core/src/` that isn't the
[component system](component-system.md), [interop layer](interop.md),
[runtime/device backends](runtime-and-devices.md), or the
[self-extending systems](llm-providers.md) (each has its own doc).

## Crate root (`core/src/lib.rs`)

The module tree: `orchestrator, bbir, cluster, component, runtime, interop, data,
utils, models, llm, intent, interpret, diagnostics, synthesis, agent, autotune,
batch_predict` (`core/src/lib.rs:1-17`).

- **`TensorHandle`** (`lib.rs:37-46`) owns a heap `*mut dlpack::ManagedTensor` by raw
  pointer rather than by value — necessary because DLPack's `deleter(self)` contract
  must be invoked against the exact address the original producer allocated, which
  matters once tensors are imported from a foreign producer like PyTorch (see
  [Interop](interop.md)). `unsafe impl Send + Sync` since the pointer is exclusively
  owned and never aliased; `Drop` calls the deleter function pointer.
- **`Tensor`** = `Arc<TensorHandle>` (`lib.rs:49`) — the crate-wide zero-copy tensor
  handle, shared across the async runtime.
- **`Result<T>`** (`lib.rs:52`) aliases `std::result::Result<T,
  interop::protocol::BrainBuilderError>` — the crate-wide error type.
- **`AppContext`** (`lib.rs:55-66`) is the shared application state, passed explicitly
  (never a global): `registry: RwLock<ComponentRegistry>`, `arena: Arc<SharedArena>`,
  `provenance: ProvenanceStore`, `experiments: ExperimentLog`, `checkpoints_dir:
  PathBuf`. `checkpoint_path(graph_id)` (`lib.rs:69-71`) builds
  `<checkpoints_dir>/<graph_id>.pt`.

Cargo feature flags (`cpu`, `gui`, `gpu`, `onnx`) are markers with no `#[cfg]` gating
inside `core/src` except `gpu`'s optional dependencies — see
[Runtime & Devices](runtime-and-devices.md) for what each feature actually enables.

## Orchestrator (`core/src/orchestrator.rs`)

`Orchestrator` is "the central orchestrator that receives BBIR graphs from the GUI and
manages the entire execution lifecycle" (`orchestrator.rs:13-23`). It holds
`context: Arc<AppContext>`, `trainer_controller: TrainerController`, `python:
PythonBridge`, and `preferred_gpu: Mutex<Option<String>>`.

`Orchestrator::new(components_dir)` (`orchestrator.rs:26-58`) builds a `SharedArena`,
loads every `.edn` file under `components_dir` into a `ComponentRegistry`, opens
`provenance.sqlite3` and `experiments.sqlite3` under the project root, creates
`checkpoints/`, and starts a `PythonBridge` and `TrainerController`.

Public API:

| Method | Purpose |
|---|---|
| `set_preferred_gpu(name)` | Normalizes/clears the GPU pick; forwards a torch-side device choice to `PythonBridge::set_device` (any picked adapter maps to torch's `"cuda"` device-type namespace, shared by ROCm and NVIDIA) |
| `validate(graph)` | Structural/shape-only check via `component::validation::validate_graph`, no execution — instant canvas feedback |
| `execute_graph(graph)` (async) | Validates, captures a Nix environment snapshot if none is pinned, compiles via `runtime::scheduler::compile`, loads data via `data::source::load_dataset`, and calls `trainer.fit(plan, data)` — the main "train a graph" entrypoint |
| `predict(graph, batch)` / `predict_output(graph, batch)` | Forward-only inference. `predict_output` resolves the real output unambiguously via `ExecutionPlan::output_port()`, replacing an earlier unsound heuristic that guessed the smallest tensor whose length divided the row count (broke for sequence models and multi-class classifiers) |
| `has_checkpoint(graph_id)` | Whether a checkpoint file exists for this graph |
| `install_component(path)` | Reads an `.edn` file from disk, parses it, inserts into the registry under a write lock |
| `component_summaries()` | The current source of truth the GUI palette/inspector uses (ports + roles + hyperparameter schema) |

Every registry/lock access maps a poisoned lock to a clean `Err` rather than
panicking; `orchestrator.rs`'s `chaos_tests` module regression-tests this specifically
for `preferred_gpu`.

## BBIR: the graph intermediate representation (`core/src/bbir.rs`)

BBIR ("BrainBuilder Intermediate Representation") is the canonical, serializable graph
format — what gets saved to disk, sent over Tauri IPC as JSON, and authored by the LLM
graph-generation pipeline. `CURRENT_BBIR_SCHEMA_VERSION = 1` (`bbir.rs:18`); a saved
graph with no `:schema-version` field is treated as version 1 rather than an error.

| Type | Fields | Notes |
|---|---|---|
| `BBIRGraph` | `schema_version, graph_id, name, nodes, edges, training` | The whole document |
| `BBIRNode` | `id, component, label, hyperparams, ports, position` | `component` is a name reference into the registry |
| `NodePosition` | `{x, y}` | Canvas layout coordinates, round-tripped so reloading a saved graph doesn't scatter nodes |
| `PortInfo` | `input_ports, output_ports` | The node's own wired port name lists (distinct from the descriptor's richer `PortSpec`) |
| `BBIREdge` | `from_node, from_port, to_node, to_port` | |
| `TrainingConfig` | `loss, optimizer, trainer_type, hyperparams, data_source, reproducibility` | |
| `DataSourceConfig` | `source_type, path_or_uri, batch_size, preprocessing`, plus type-specific fields | `source_type` is one of `"file"`, `"image_folder"`, `"text_sequence"`, `"text_column"` |
| `ReproducibilityConfig` | `nix_expression, seed` | |

**The last-column-is-target convention.** `DataSourceConfig` documents that if no
label column is explicitly named, the last column of a dataset is treated as the
training target (`bbir.rs:221-223`). This convention recurs across `data/source.rs`,
`data/vision.rs`, `data/tabular_text.rs`, and `interpret.rs` — it's a real, load-bearing
assumption in several places, not just a default.

**Vocab-size coordination gap.** `DataSourceConfig` documents that a `text_sequence`
dataset's `vocab_size` must be manually kept in sync with the graph's `embedding` node
hyperparameter — the engine doesn't yet auto-wire dataset vocab size into a node's
hyperparameters (`bbir.rs:196-202`). The GUI's [Data panel](gui-panels.md#data-panel)
works around this with a `useEffect` that keeps every embedding node's `vocab_size` in
lockstep with the dataset config.

**EDN round-trip.** `BBIRGraph::from_edn`/`to_edn` use `edn_rs`. Most nested structs
derive `edn_derive::Serialize/Deserialize`, but `BBIRGraph`, `BBIRNode`,
`TrainingConfig`, and `PreprocStep` hand-write their EDN trait impls to handle optional
and nested fields the derive macro can't express. `serde::{Serialize,Deserialize}` is
deliberately *not* imported by name alongside `edn_derive`'s traits — both produce
inherent-looking `serialize`/`deserialize` methods, and having both in scope makes
`x.serialize()` ambiguous (rustc E0034). Round-trip correctness (including EDN↔JSON
edge cases like symbolic keyword casing) is covered by `core/tests/bbir_roundtrip.rs`
and a `proptest`-based `core/tests/bbir_roundtrip_proptest.rs`.

## Intent: task-first graph authoring (`core/src/intent.rs`)

Not related to `interpret.rs` despite the similar name. The Intent layer turns a plain
goal ("I have a folder of photos labeled cat/dog, tell them apart") plus a pointer at
real data into a validated, trainable `BBIRGraph`, sized to the data, with a
plain-English rationale — deliberately scoped as "a small, honest set of architectures
— the floor that makes the existing engine reachable, not a pretense of covering every
model ever" (`intent.rs:23-26`).

Flow (split into async/sync halves because the registry's `std::sync::RwLock` guard
must never be held across an `.await`):

1. `inspect_data(&IntentRequest) -> Result<DataShape>` (async) — reuses the trainer's
   real ingestion code per source type (`image_folder` scans class subfolders,
   `text_column` builds a bag-of-words via `data::tabular_text`, `file` reads column
   names and distinct label values) so the proposal can't drift from what training will
   actually see.
2. `finalize_proposal(request, shape, registry) -> Result<ProposedModel>` (sync) —
   computes `num_outputs` (class count, minimum 2 for classification), assembles a
   fixed 2-layer MLP (`linear → relu → linear`, hidden width `sqrt(feature_count)*4`
   clamped to `[16, 256]`), and validates it through the exact
   `component::validation::validate_graph` real training uses.

Default hyperparameters chosen by the Intent layer: Adam, lr=0.001, 40 epochs,
batch_size=32.

`diagnose_data` runs the [diagnostics](#diagnostics-core-srcdiagnosticsrs) rules
(class balance, feature/target leakage) against real data before training even starts.

**Transfer learning.** `propose_transfer_model`/`TransferRequest` build a frozen
`lora_linear` backbone seeded from a chosen 2-D tensor in a `.safetensors` file
(`trainable: false`) feeding a fresh trainable linear head — real LoRA-style adaptation
of a single pretrained weight matrix. The code is explicit about the boundary here:
using a *full* pretrained network as a frozen feature extractor is "the documented next
step that builds on this same seam (`scheduler::load_preset_weights`)," not something
implemented yet (`intent.rs:229-234`).

## Interpretability (`core/src/interpret.rs`)

Despite the name, this is permutation-style *feature importance*, not intent parsing.
`feature_importance(orchestrator, graph, batch)` ranks every input column (all but the
last, the target by convention) by how much predictions shift when that column is
decoupled from its row — using a **deterministic one-row cyclic rotation**
(`rotate_column`) rather than a random shuffle, specifically so a user gets the same
ranking every time they ask, and to avoid adding a `rand` dependency.

## Diagnostics (`core/src/diagnostics.rs`)

Pure, deterministic, I/O-free rule functions, deliberately kept side-effect-free for
unit-testability. Each returns a `Diagnostic{severity, title, explanation,
suggestion}`.

- `analyze_class_balance` — flags ≥80% single-class imbalance, classes with fewer than
  5 examples, and overall too-little-data.
- `analyze_feature_target_leakage` — flags a feature with Pearson correlation ≥0.999 to
  the target as likely leakage.
- `analyze_loss_curve` — flags NaN/divergence, loss increasing, a barely-changed
  (<2% relative drop) curve, and affirms a healthy ≥20% drop.

All I/O (reading data, streaming losses) is kept in the callers
(`intent::diagnose_data`, the GUI command layer) — this module never touches disk or
the network.

## Batch prediction (`core/src/batch_predict.rs`)

`run_batch_predict(orchestrator, graph, dataset_path, output_path)` streams a dataset
file in chunks via `data::source::load_all`, runs each chunk through
`interpret::predict_flat`, and writes `<input columns...>,prediction` CSV rows to disk
— never holding the whole file in memory. No target column is assumed or dropped
(unlike `feature_importance`) — the dataset is expected to already be exactly the
feature columns the graph was trained on. Multi-value-per-row outputs (a classifier's
`[batch, num_classes]`) are written as the argmax class index rather than a raw value.

## Data & ETL (`core/src/data/`)

- **`etl.rs`** — `apply_steps(batch, steps)` folds a list of `PreprocStep`s over an
  Arrow `RecordBatch`. Only two ops are implemented against real Arrow kernels:
  `normalize` (z-score a float32 column; a zero-variance column is left unchanged
  rather than dividing by zero) and `cast` (wraps `arrow::compute::cast`). Any other op
  name returns a clear `ConfigError` instead of silently no-op'ing — image/text ops
  like `resize` or `tokenize` would need a real codec/tokenizer dependency this crate
  doesn't have yet, so rather than fake them, they error.
- **`source.rs`** — DataFusion-backed CSV/Parquet reading. `DataIterator::reset()`
  exists because of a real historical bug: without it, `StandardTrainer::fit`'s
  per-epoch loop only ever consumed batches once, so `epochs` silently did nothing past
  epoch 0 for any dataset small enough to fit in one DataFusion chunk.
  `load_dataset(config)` dispatches on `data_source.source_type`: `text_sequence` (via
  `data::text::Vocab`, windowed into `(context, next_token)` pairs), `image_folder`
  (via `data::vision::load_image_folder`), `text_column` (bag-of-words via
  `data::tabular_text`), or `file` (DataFusion read + `etl::apply_steps`). An optional
  `shuffle` hyperparameter (default false) reorders batches per epoch. `load_all(path)`
  reads an entire file into memory, used by batch prediction.

## Provenance (`core/src/utils/provenance.rs`)

`ProvenanceStore` wraps a bundled `rusqlite` connection (no system libsqlite3 needed).
Schema: `provenance(id, graph_version, node_id, sample_id, step, logged_at)`. Genuinely
append-only — only `INSERT` is ever issued anywhere in the file, no `UPDATE`/`DELETE`
— recording the exact data-sample → weight-update lineage for every training step. A
single shared instance lives at `<project_root>/provenance.sqlite3`, opened once in
`Orchestrator::new`.

## Nix environment capture (`core/src/utils/nix.rs`)

`capture_nix_environment()` produces a syntactically valid Nix attribute-set string
(explicitly *not* a buildable derivation — mapping an arbitrary pip/cargo package to
the matching nixpkgs derivation name isn't something that can be done generically) that
pins exact versions: Rust crates parsed directly out of the workspace's `Cargo.lock`
(a hand-rolled line parser, avoiding a TOML dependency) and Python packages via a real
`python -m pip freeze` subprocess. Called from `Orchestrator::execute_graph` whenever a
graph's `ReproducibilityConfig.nix_expression` is unset, logging the captured
environment as a reproducibility fallback rather than blocking training.

## Known gaps in this layer

- CUDA kernel execution (`core/src/runtime/cuda_backend.rs`) has real `todo!()` stubs —
  see [Runtime & Devices](runtime-and-devices.md#cuda-backend-cuda-feature).
- Transfer learning only supports seeding from a single pretrained weight matrix, not a
  full frozen network (`intent.rs:229-234`).
- Dataset vocab size and graph embedding size must be kept in sync manually
  (`bbir.rs:196-202`).

See also: [Component System](component-system.md) for how `ComponentRegistry` and
descriptors work, [Runtime & Devices](runtime-and-devices.md) for how a compiled
`ExecutionPlan` actually runs, and [Interop](interop.md) for how training reaches real
PyTorch.
