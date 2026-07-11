---
name: harness
description: Use universal-harness (this repo's ../universal-harness/) to call BrainBuilder's own CLI, API, or GUI surfaces as agent-callable operations, or to harness any other software the same way. Trigger on "harness this", "wrap X for agents", or when you need to script/automate part of BrainBuilder itself outside the chat session.
---

# Harness (BrainBuilder)

The real toolkit lives at [`universal-harness/`](../../universal-harness/skills/harness/SKILL.md)
at the repo root — read that file for the full workflow (introspect → list →
call → mcp). This file just wires it into BrainBuilder specifically.

## Pre-built BrainBuilder connectors

`universal-harness/connectors/brainbuilder/` has what's committed so far:

- `pipeline-cli.manifest.json` — real, live-verified. Introspected from
  `scripts/ci/pipeline.mjs --help` and confirmed to actually run the pipeline
  (`harness call ... --op pipeline`) end to end.
- `predict-api.openapi.json` + `predict-api.manifest.json` — hand-authored
  from reading `gui/src-tauri/src/predict_server.rs`'s real route directly
  (it doesn't self-serve a spec), structurally correct, but **not yet
  live-verified**: `live-predict-proof.mjs` (same folder) hits a real,
  pre-existing bug in BrainBuilder's own Tauri setup unrelated to the
  harness — `tauri dev`'s window serves Tauri v2's JS IPC bridge
  (`window.__TAURI_INTERNALS__`) while `gui/src-tauri/tauri.conf.json` and
  both `Cargo.toml`/`package.json` are pinned to Tauri v1, so every IPC
  command (not just this one) currently returns "not found". Fix that
  version/config mismatch first, then rerun `node connectors/brainbuilder/live-predict-proof.mjs`
  from `universal-harness/` — it regenerates `predict-api.manifest.json`
  against the real server URL and calls it for real as its own proof.
- No committed `gui.manifest.json` — BrainBuilder's DOM changes with every
  new tutorial/feature, so generate it fresh (command below) rather than
  trust a stale one.

Regenerate:

```
cd universal-harness
node dist/src/cli.js introspect-cli --command "node ../scripts/ci/pipeline.mjs" --out connectors/brainbuilder/pipeline-cli.manifest.json
node dist/src/cli.js introspect-gui --cdp-url http://localhost:9222 --url-includes "http://tauri.localhost/" --name "BrainBuilder GUI" --out connectors/brainbuilder/gui.manifest.json
```

(`--url-includes` matters: BrainBuilder's dev window exposes more than one
CDP page target — e.g. an `assistant.html` secondary window — and without it
`introspect-gui` may attach to the wrong one.)

## Use them

```
node universal-harness/dist/src/cli.js mcp \
  --manifest universal-harness/connectors/brainbuilder/pipeline-cli.manifest.json \
  --name brainbuilder
```

registers BrainBuilder's own CI pipeline as MCP tools any agent can call —
including, recursively, Claude Code itself via `.mcp.json` at the repo root.
Add `--manifest .../gui.manifest.json` once you've regenerated it fresh.
