# BrainBuilder

BrainBuilder is a desktop application for visually building, training, predicting
with, and extending neural network graphs — backed by a real Rust orchestration engine
and a real PyTorch training backend, not a simulation or a mockup. You drag components
onto a canvas, wire them together, point at a dataset, and train — with a live loss
chart, checkpoint management, prediction, and interpretability tooling built in. Where
BrainBuilder goes further than a typical graph editor is that the library of
components, the graphs themselves, and even the app's own codebase are all things the
app can extend: describe a model in plain English and get a trained graph, describe a
new layer and get a real, sandboxed, installable component, or hand the app a coding
task and let a self-building agent implement it inside an isolated, test-gated git
worktree.

**Full documentation:** [docs/README.md](docs/README.md) is the complete, in-depth
reference for every system described below — architecture, every panel, every
component, every subprocess bridge, and every self-extending feature, each with exact
file/line references into the source.

## What you can do with it

- **Build a model visually.** Drag components (linear layers, attention, convolution,
  embeddings, LoRA adapters, and more) onto a `reactflow` canvas, wire ports together,
  and configure loss/optimizer/learning rate/epochs — or start from one of five
  built-in [templates](docs/templates.md) (MLP classifier, transformer attention
  stack, text sentiment classifier, next-word predictor, speculative-decoding
  drafter).
- **Build a model from a goal, not a graph.** Point the [Intent panel](docs/gui-panels.md#intent-panel)
  at a folder of labeled photos or a CSV and describe the task; it inspects the real
  data and proposes a validated, appropriately-sized architecture with a plain-English
  rationale — including transfer learning from a pretrained weight file.
- **Describe a model or a component in plain English.** The
  [LLM Author panel](docs/gui-panels.md#llm--synthesis--agent-gui-side-only) generates
  a whole graph from a description; the
  [Synthesize panel](docs/synthesis.md) generates a brand-new component — descriptor
  and Python kernel — and proves it works in a sandboxed smoke test before it's ever
  installed. Works against a local [Ollama](docs/llm-providers.md#ollamaprovider)
  install or a hosted [OpenCode Go](docs/llm-providers.md#opencodeprovider) account,
  interchangeably.
- **Train for real.** Real PyTorch autograd, real optimizers, live loss streaming, GPU
  device selection (including AMD via Vulkan/wgpu), checkpoint save/load/rollback,
  early stopping, and automatic [hyperparameter and architecture search](docs/autotune.md).
- **Train across your own devices.** A peer-to-peer [device cluster](docs/cluster-and-distributed.md)
  with real distributed gradient averaging over libp2p — no cloud account required.
- **Predict and serve.** Run predictions on a preview batch, export a full-dataset CSV,
  or start a local `/predict` HTTP server directly from the app. Rank feature
  importance for tabular models.
- **Extend the app itself.** The [self-building agent](docs/agent.md) drives an
  external coding agent against BrainBuilder's own repository, confined to an isolated
  git worktree with a test gate before any merge — with three levels of autonomy from
  fully human-reviewed to fully automatic.
- **Serve with speculative decoding.** [DSpark](docs/dspark-system.md) is a standalone
  draft/verify inference engine with dynamic, hardware-aware draft-length tuning,
  separate from the trainer by design.

## How it's built

| Layer | Technology | Docs |
|---|---|---|
| Training/orchestration engine | Rust (`core/`, crate `brainbuilder-core`) | [Core Engine](docs/core-engine.md) |
| Desktop shell | Tauri 1.x + React/TypeScript (`gui/`) | [GUI Architecture](docs/gui-architecture.md) |
| Component library | EDN descriptors + Python/PyTorch (`components/`) | [Component System](docs/component-system.md), [Component Library](docs/component-library.md) |
| Real training math | PyTorch, via a persistent sandboxed Python worker | [Interop](docs/interop.md) |
| Symbolic differentiation / config DSLs | Racket, Clojure (subprocess bridges) | [Interop](docs/interop.md) |
| Distributed training | libp2p peer-to-peer mesh | [Cluster & Distributed Training](docs/cluster-and-distributed.md) |
| Speculative-decoding serving | Standalone Python engine (`dspark_system/`) | [DSpark](docs/dspark-system.md) |
| Agent-callable automation | Generic CLI/API/GUI-to-MCP wrapper (`universal-harness/`) | [Universal Harness](docs/universal-harness.md) |

See [Architecture](docs/architecture.md) for how these pieces fit together end to end,
including the full request lifecycle from a canvas edit to a trained checkpoint.

## Design principles

- **Everything untrusted or generated is sandboxed.** Racket, Clojure, the Python
  worker, synthesized components, and the self-building agent's coding process all run
  through the same capability-based sandbox — deny-by-default filesystem access, no
  network unless granted, hard timeouts, and (platform-dependent) memory ceilings. See
  [the nervous-system sandbox](docs/runtime-and-devices.md#the-nervous-system-sandbox).
- **Graphs and components are data, not code.** A model is a plain JSON/EDN document; a
  component is a descriptor plus a small Python file. This is what makes hot-installing
  a synthesized component, or an LLM-authored graph, possible with no recompilation.
- **The GUI has no hardcoded panels.** Every panel — built-in or a runtime-loaded
  plugin — registers into the same [widget registry](docs/gui-architecture.md#gui-srcwidgets--the-widgetregistry-system)
  through the same capability-gated API.
- **Nothing pretends to work.** Where a real dependency is genuinely missing (a CUDA
  toolkit, the `opencode` CLI, a tokenizer library), the code returns a clear, named
  error instead of a silent no-op — and every doc under [docs/](docs/README.md) is
  honest about current gaps, not just finished features.

## Getting started

```bash
cargo build --workspace --no-default-features --features brainbuilder-core/cpu,brainbuilder-core/gui
cd gui && npm install && npx tauri build --debug   # or `npx tauri dev`
```

Run the automated verification suite:

```powershell
pwsh scripts/verify.ps1
```

See [Scripts & Testing](docs/scripts-and-testing.md) for what each script and test
suite covers, and the [Live Verification Checklist](docs/VERIFICATION.md) for the
handful of paths (live LLM network calls, the `opencode` CLI, specific GPU hardware)
that need a real machine and can't be proven in an automated sandbox.

## Documentation

Start at **[docs/README.md](docs/README.md)** for the full map. Highlights:

- [Architecture](docs/architecture.md) — the system end to end
- [Core Engine](docs/core-engine.md) · [Component System](docs/component-system.md) ·
  [Component Library](docs/component-library.md)
- [Interop](docs/interop.md) · [Runtime & Devices](docs/runtime-and-devices.md) ·
  [Cluster & Distributed Training](docs/cluster-and-distributed.md)
- [LLM Providers](docs/llm-providers.md) · [Component Synthesis](docs/synthesis.md) ·
  [Self-Building Agent](docs/agent.md) · [Autotune](docs/autotune.md)
- [GUI Architecture](docs/gui-architecture.md) · [GUI Panels](docs/gui-panels.md) ·
  [Templates](docs/templates.md)
- [DSpark Speculative Decoding](docs/dspark-system.md)
- [Universal Harness](docs/universal-harness.md) · [Scripts & Testing](docs/scripts-and-testing.md)
- [Glossary](docs/glossary.md) · [Live Verification Checklist](docs/VERIFICATION.md) ·
  [Build History & Engineering Log](docs/build-history.md)
