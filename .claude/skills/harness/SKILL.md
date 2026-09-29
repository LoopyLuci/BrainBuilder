---
name: harness
description: Use universal-harness (this repo's ../universal-harness/) to call BrainBuilder's own CLI, API, or GUI surfaces as agent-callable operations, or to harness any other software the same way. Trigger on "harness this", "wrap X for agents", or when you need to script/automate part of BrainBuilder itself outside the chat session.
---

# Harness (BrainBuilder)

The real toolkit lives at [`universal-harness/`](../../universal-harness/skills/harness/SKILL.md)
at the repo root — read that file for the full workflow (introspect → list →
call → mcp). This file just wires it into BrainBuilder specifically.

## Pre-built BrainBuilder connectors

`universal-harness/connectors/brainbuilder/` — all three live-verified end to
end (no mocks, real IPC, real HTTP, real DOM):

- `pipeline-cli.manifest.json` — introspected from `scripts/ci/pipeline.mjs
  --help`, confirmed by actually running the pipeline through it
  (`harness call ... --op pipeline`).
- `predict-api.openapi.json` + `predict-api.manifest.json` — hand-authored
  from `gui/src-tauri/src/predict_server.rs`'s real route, confirmed by
  starting a real predict server via Tauri IPC and getting real prediction
  tensors back (`live-predict-proof.mjs`, same folder).
- `gui.manifest.json` — 75 real operations discovered live from
  BrainBuilder's actual running window (tabs, canvas controls, template
  buttons, ...), confirmed by actually clicking one
  (`harness call ... --op tab-templates`).

Regenerate any of them (DOM/routes/flags change as the app evolves):

```
cd universal-harness
node dist/src/cli.js introspect-cli --command "node ../scripts/ci/pipeline.mjs" --out connectors/brainbuilder/pipeline-cli.manifest.json
node dist/src/cli.js introspect-gui --cdp-url http://localhost:9222 --url-includes "localhost:5173" --name "BrainBuilder GUI" --out connectors/brainbuilder/gui.manifest.json
node connectors/brainbuilder/live-predict-proof.mjs   # regenerates + live-verifies predict-api.manifest.json in one shot
```

(`--url-includes "localhost:5173"` matches BrainBuilder's real dev-server
URL — the app's `devPath` in `tauri.conf.json`. Only needed if more than one
CDP page target is ever exposed again.)

### The WebView2-profile bug (fixed, worth knowing about)

Until 2026-07-11, **every** Tauri IPC command failed "not found" in `tauri
dev` — not a harness bug, a real BrainBuilder one. WebView2/wry default every
app to one *shared* profile directory (`%LOCALAPPDATA%\EBWebView`), and every
Tauri app also shares the fixed `http://tauri.localhost/` origin, so a
service worker registered by a *different* local Tauri app was intercepting
BrainBuilder's own page loads and serving its own UI in place of
BrainBuilder's — different app, different `invoke_handler`, hence "not
found" for literally everything. Fixed in `gui/src-tauri/src/main.rs`
(`fn main()`, top) by giving BrainBuilder its own
`WEBVIEW2_USER_DATA_FOLDER`. If IPC commands ever start failing "not found"
again for no obvious reason, check that fix hasn't regressed before assuming
it's something else.

## Use them

```
node universal-harness/dist/src/cli.js mcp \
  --manifest universal-harness/connectors/brainbuilder/pipeline-cli.manifest.json \
  --manifest universal-harness/connectors/brainbuilder/gui.manifest.json \
  --name brainbuilder
```

registers BrainBuilder's own CI pipeline and live GUI as MCP tools any agent
can call — including, recursively, Claude Code itself via `.mcp.json` at the
repo root. The predict-api manifest's `baseUrl` is an ephemeral, OS-assigned
port (regenerated fresh by `live-predict-proof.mjs` each time the server
starts), so it's not meant to be merged into a long-lived MCP server the same
static way.
