# Glossary

Terms used throughout BrainBuilder and this documentation. The GUI has the same
glossary built in (`gui/src/help/glossary.ts`), surfaced via `?` `HelpTip` buttons
across most panels and in the [Learn tab](gui-panels.md#tutorial--learn-tab-guisrctutorial).

## BrainBuilder-specific terms

**BBIR** — "BrainBuilder Intermediate Representation," the canonical, serializable
graph format (nodes, edges, training config). See [Core Engine](core-engine.md#bbir-the-graph-intermediate-representation).

**Component** — a reusable graph operation described by an `.edn` descriptor plus
(usually) a Python implementation — a linear layer, an attention block, an optimizer
step. See [Component System](component-system.md).

**Component descriptor** — the parsed form of a component's `.edn` file: its ports,
hyperparameters, implementation language/entry point, and compatibility. See
[Component System](component-system.md#the-descriptor-core-srccomponentdescriptorrs).

**Port role (`Data` / `Parameter`)** — whether a component port carries an ordinary
tensor (`Data`) or a learnable weight the trainer auto-initializes and optimizes
(`Parameter`). See [Ports and roles](component-system.md#ports-and-roles).

**The nervous system** — BrainBuilder's capability-based sandbox for every subprocess
it spawns (Racket, Clojure, the Python worker, the self-building agent's `opencode`
process): deny-by-default filesystem access, no network unless granted, a hard
timeout, and (platform-dependent) a memory ceiling. See
[Runtime & Devices](runtime-and-devices.md#the-nervous-system-sandbox).

**The gauntlet** — the three-gate validation a synthesized component must clear
(parse → structural validation → sandboxed smoke test) before it can be installed. See
[Component Synthesis](synthesis.md#the-gauntlet).

**Intent layer** — the task-first on-ramp that turns a plain goal ("classify these
photos") plus real data into a validated, trainable graph. See
[Core Engine](core-engine.md#intent-task-first-graph-authoring-core-srcintentrs).

**Autonomy mode** — how much the [self-building agent](agent.md) is allowed to do
unsupervised: propose-approve (human reviews every merge), auto-apply (merges
automatically on green tests), or full (merges regardless of test outcome).

**Widget / widget registry** — the GUI's single extension mechanism; every panel,
built-in or plugin, is a `WidgetDef` registered into a slot. See
[GUI Architecture](gui-architecture.md#gui-srcwidgets--the-widgetregistry-system).

**Selector (`provider:model`)** — the string format used everywhere an LLM call is
made, e.g. `ollama:llama3` or `opencode:opencode-go/claude-sonnet-5`. See
[LLM Providers](llm-providers.md#providerregistry-core-srcllmregistryrs).

**SPS (System Performance vs Speed)** — the metric [DSpark's](dspark-system.md)
`SPSManager` uses to dynamically adjust speculative draft length based on live
hardware load. Not an abbreviation for "speculation-per-step," despite the acronym's
apparent similarity.

## General machine-learning terms

**Neural network** — a model made of layered mathematical operations (components, in
BrainBuilder's terms) that learns patterns from data by adjusting numeric weights.

**Architecture** — the specific arrangement of components/layers in a model — how many,
what kind, and how they're connected.

**Epoch** — one full pass through the training dataset.

**Loss** — a single number measuring how wrong the model's current predictions are;
training tries to make this number go down.

**Learning rate (lr)** — how big a step the optimizer takes toward reducing loss on
each update. Too high and training diverges; too low and it's slow to converge.

**Optimizer** — the algorithm that updates weights based on the loss gradient (e.g.
SGD, Adam).

**Batch size** — how many examples are processed together before one weight update.

**Overfitting** — a model that has memorized the training data's quirks rather than
learned a generalizable pattern, and performs worse on new data as a result.

**Early stopping** — halting training once the loss stops meaningfully improving, to
avoid overfitting and wasted compute. Configured via the `patience` hyperparameter in
the [Data panel](gui-panels.md#data-panel).

**Regression vs. classification** — regression predicts a continuous number;
classification predicts which of a fixed set of categories an input belongs to.

**Transfer learning** — reusing weights from a model already trained on other data
(usually a much larger dataset) as a starting point, rather than training from
scratch. See [Intent layer](core-engine.md#intent-task-first-graph-authoring-core-srcintentrs).

**Data augmentation** — synthetically expanding a training dataset (e.g. flipping or
rotating images) to improve generalization.

**Feature importance** — a ranking of which input columns most influence a model's
predictions. See [Interpretability](core-engine.md#interpretability-core-srcinterpretrs).

**Attention** — a mechanism that lets a model weigh different parts of its input
differently depending on context, the core building block of transformer
architectures. See the [attention component](component-library.md#catalog).

**LoRA (Low-Rank Adaptation)** — a fine-tuning technique that freezes a model's
original weights and trains a small pair of low-rank matrices alongside them, making
adaptation cheap. See the [lora_linear component](component-library.md#catalog).

**Speculative decoding** — an inference-time technique where a cheap "drafter" model
proposes several tokens ahead and an expensive "verifier" model checks them in one
pass, reducing the number of full forward passes needed to generate text. See
[DSpark](dspark-system.md).

**Checkpoint** — a saved snapshot of a model's trained weights, loadable later for
prediction or to resume training.

**Serving** — running a trained model to answer real requests (as opposed to
training it). See the [Predict panel's local server](gui-panels.md#predict-panel) and
[DSpark](dspark-system.md).

**Rollback** — restoring a previous checkpoint version, undoing a later training run's
weights. See [checkpoint version history](gui-panels.md#predict-panel).

## See also

Every term above links back to the doc where it's defined in depth. Start at
[docs/README.md](README.md) for the full map.
