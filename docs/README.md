# BrainBuilder Documentation

This is the complete technical reference for BrainBuilder: a desktop application for
visually building, training, predicting with, and extending neural network graphs,
backed by a real Rust orchestration engine and a real PyTorch training backend.

Start with the root [README.md](../README.md) for a project overview. This folder
covers every subsystem in depth.

## Map of the documentation

### Core engine (Rust, `core/`)
- [Architecture](architecture.md) — how everything fits together, end to end
- [Core Engine](core-engine.md) — orchestrator, BBIR graph format, intent layer,
  interpretability, batch prediction, data/ETL, provenance, Nix capture
- [Component System](component-system.md) — how components are described, registered,
  and validated (the `.edn` descriptor format, ports, roles)
- [Component Library](component-library.md) — every shipped component, what it computes,
  and whether it's implemented
- [Interop](interop.md) — the Python/PyTorch worker protocol, Racket bridge, Clojure
  bridge, DLPack tensor transport
- [Runtime & Devices](runtime-and-devices.md) — CPU/GPU/wgpu backends, device selection,
  the sandboxing "nervous system"
- [Cluster & Distributed Training](cluster-and-distributed.md) — the peer-to-peer device
  mesh and distributed gradient averaging

### Self-extending systems ("next-gen" phases)
- [LLM Providers](llm-providers.md) — multi-provider LLM abstraction (Ollama, OpenCode),
  credential storage, graph authoring
- [Component Synthesis](synthesis.md) — generating brand-new components from a
  description, and the safety gauntlet that gates them
- [Self-Building Agent](agent.md) — an LLM-driven coding agent that edits BrainBuilder's
  own repo inside an isolated, test-gated git worktree
- [Autotune](autotune.md) — automatic hyperparameter and architecture search

### GUI (Tauri + React, `gui/`)
- [GUI Architecture](gui-architecture.md) — the Tauri host, the widget/plugin system,
  state stores, and how a canvas action becomes a training run
- [GUI Panels](gui-panels.md) — every panel's purpose and workflow
- [Templates](templates.md) — the built-in starter graphs and the template system

### Serving & applications
- [DSpark Speculative Decoding](dspark-system.md) — the standalone draft/verify
  inference-serving engine

### Tooling
- [Universal Harness](universal-harness.md) — the agent-callable wrapper around
  BrainBuilder's CLI, HTTP, and GUI surfaces
- [Scripts & Testing](scripts-and-testing.md) — build/verify/ship scripts, CI pipeline,
  and the test suite

### Reference
- [Glossary](glossary.md) — terminology used throughout the app and these docs
- [Live Verification Checklist](VERIFICATION.md) — the manual checklist for paths that
  need live network/hardware and can't be proven in an automated sandbox
- [Build History & Engineering Log](build-history.md) — the original from-scratch build
  log: every real bug found and fixed getting this codebase from a design blueprint to
  a running, verified application

## Reading order

If you're new to the codebase, read in this order: [Architecture](architecture.md) →
[Core Engine](core-engine.md) → [Component System](component-system.md) →
[GUI Architecture](gui-architecture.md) → [GUI Panels](gui-panels.md). Everything else
is reference material you can jump to as needed — every doc links to the others where
relevant.
