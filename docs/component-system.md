# Component System

Every operation a graph node can perform — a linear layer, an attention block, an
optimizer step — is described by a **component**: a `.edn` descriptor plus (usually) a
Python implementation. This doc covers the descriptor format and the registry that
loads and validates it. For the actual catalog of shipped components, see the
[Component Library](component-library.md).

## The descriptor (`core/src/component/descriptor.rs`)

`ComponentDescriptor` is the parsed form of a `.edn` component file:

| Field | Type | Meaning |
|---|---|---|
| `id` | `String` | Content-addressed hash (`blake3` of the canonical JSON), filled in by the registry if blank or not a valid 64-char hash |
| `name` | `String` | Stable name referenced from `BBIRNode.component` |
| `meta_type` | `String` | e.g. `"stateful-module"`, `"function"`, `"pipeline"` |
| `tags` | `Vec<String>` | Free-form, e.g. `["dspark", "speculative", "drafter"]` |
| `inputs` / `outputs` | `Vec<PortSpec>` | The component's interface |
| `hyperparameters` | `HashMap<String, HyperParameterDef>` | Editable config exposed in the Inspector |
| `implementations` | `Vec<Implementation>` | Which language/entry point actually runs this component |
| `compatibility` | `Compatibility` | Supported devices, dtypes, whether autograd applies |
| `tests` | `Vec<ComponentTest>` | Self-contained shape/value tests |

EDN keys use kebab/namespaced forms (`:component/id`, `:interface/inputs`,
`:meta-type`), mapped through a manual `EdnDeserialize` impl rather than a pure derive.

### Ports and roles

`PortSpec` — `name`, `tensor: TensorSpec`, **`role: PortRole`**, `trainable: bool`
(default `true`), `zero_init: bool` (default `false`).

`PortRole` is the field that makes the trainer able to auto-manage weights without the
graph author wiring them by hand:

- **`Data`** (default) — an ordinary tensor flowing through the graph (activations,
  the dataset batch).
- **`Parameter`** — a learnable weight. The trainer detects parameter ports as inputs
  no other node produces, lazily initializes them (`torch.randn`, or all-zero if
  `zero_init: true`), and threads them across the whole epoch×batch loop so they
  actually accumulate gradient updates instead of resetting every step.

`trainable: false` marks a frozen parameter — used by `lora_linear`'s base `weight`
port, which stays frozen while its low-rank adapters (`lora_a`, `lora_b`) train.
`zero_init: true` marks a parameter that must start at zero — used by LoRA's `lora_b`
so the adapter is an exact no-op at the start of training (see
[Component Library](component-library.md#lora_linear)).

`ComponentDescriptor::trainable_parameter_ports()`/`zero_init_parameter_ports()`
(`descriptor.rs:343-359`) are what the scheduler queries to decide which node inputs
to auto-initialize and hand to the optimizer.

### Shapes

`TensorSpec` is `{shape: ShapeExpr, dtype: DataType, sparsity}`. `ShapeExpr` is either
`Fixed(Vec<isize>)` or `Symbolic(serde_json::Value)` — shapes may contain symbolic
dimensions like `:batch`/`:in-features` or expressions like `(:- :h :kh 1)`, resolved
later by `runtime::scheduler::resolve_shape_dims_partial` against a node's actual
hyperparameters. `DataType` is one of Float32, Float16, Int32, Int64, Bool — note that
in practice, tensor exchange across the Rust↔Python boundary is float32 end-to-end (see
[Interop](interop.md)); components that logically deal in integers (like `embedding`'s
token ids) accept a float32 tensor and cast internally.

### Implementations

`Implementation{language, path, entry}` is the dispatch target for a node: `language`
is `"python"` for every shipped component today (`conv2d.edn` additionally *declares*
a `"rust"` implementation entry, but no matching `.rs` file exists yet — aspirational,
not wired). `entry` names the function the runtime calls (`"forward"` by default).

### `ComponentSummary` — the GUI-facing DTO

`ComponentDescriptor::summary()` produces a camelCase-friendly `ComponentSummary` /
`PortSummary` / `HyperParamSummary`, decoupling the frontend from raw EDN key naming.
This is what `Orchestrator::component_summaries()` returns and what the
[Component Palette](gui-panels.md#component-palette) and
[Inspector](gui-panels.md#inspector-panel) render from.

## The registry (`core/src/component/registry.rs`)

`ComponentRegistry { by_hash: HashMap<String, ComponentDescriptor>, by_name:
HashMap<String, String> }`.

- `load_from_dir(dir)` reads every `*.edn` file in a directory and parses+inserts each
  — this is how `Orchestrator::new` populates the registry from `components/` at
  startup.
- `insert(desc)` computes a `blake3` content hash of the descriptor's canonical JSON as
  `id` if one isn't already present, and indexes by both hash and name.
- `get_by_name` / `get_by_hash` / `list_names` / `summaries()` (sorted by name for
  stable palette ordering) round out the read API.

The registry is wrapped in `AppContext.registry: RwLock<ComponentRegistry>` — a plain
`std::sync::RwLock`, which is why every Tauri command that both reads it and awaits an
LLM call (graph authoring, synthesis) is careful to drop the guard before the `.await`
rather than holding a non-`Send` guard across it.

## Validation (`core/src/component/validation.rs`)

Two entry points:

- **`validate_descriptor_self(descriptor)`** — self-consistency checks on a single
  descriptor independent of any graph: non-empty name, at least one output, unique
  non-empty port names per side. This is the gate a freshly
  [synthesized](synthesis.md) component's descriptor must clear before its kernel ever
  runs.
- **`validate_graph(graph, registry)`** — checks every node's `component` resolves in
  the registry (a fix for a prior bug where unconnected/single-node graphs skipped this
  check entirely), then for every edge checks that source/target nodes and components
  exist, source/target ports exist, dtypes match, and shapes resolve via
  `scheduler::resolve_shape_dims_partial`. This is exactly what the GUI's "Validate"
  button and `Orchestrator::validate` call, and what LLM-authored graphs (see
  [LLM Providers](llm-providers.md#graph-authoring-pipeline)) are checked against
  before they're ever handed to execution.

## Generating a component from an example (`core/src/component/designer.rs`)

`generate_from_example(input_shape, output_shape)` is the visual designer's "infer a
component from an example" flow: it emits a fresh EDN descriptor (random
`component/id` via UUID, `meta-type "function"`, single-input/single-output float32)
proven by a test to round-trip through the real `edn_rs` deserializer for
`ComponentDescriptor`. This is a separate, simpler mechanism from
[synthesis](synthesis.md), which also generates a real Python kernel from a natural
language description and runs it through a sandboxed smoke test.

## How a component gets from disk to the canvas

1. At startup, `Orchestrator::new` calls `ComponentRegistry::load_from_dir(components/)`
   — every shipped `.edn` file becomes a `ComponentDescriptor`.
2. The GUI calls `get_component_descriptors`, which serializes each descriptor's
   `ComponentSummary` for the [Component Palette](gui-panels.md#component-palette).
3. Dragging a component onto the canvas creates a `BBIRNode` referencing the
   component by name, with hyperparameters seeded from the descriptor's defaults.
4. On "Validate"/"Export & Train", `convertToBBIR` builds the full `BBIRGraph`, and
   `component::validation::validate_graph` checks it against the live registry.
5. At execution time, the scheduler resolves each node's `Implementation` and, for
   Python components, calls the [persistent worker](interop.md#python-worker-componentspython_bb_workerpy)
   with the node's inputs, parameters, and hyperparameters.

A component can also be added **hot**, with no restart: via `install_component` (a raw
`.edn` file on disk) or via the [synthesis pipeline](synthesis.md)'s
`install_synthesized_component`, both of which call
`Orchestrator::install_component`/`registry.insert` directly, after which it
immediately appears in `component_summaries()`.
