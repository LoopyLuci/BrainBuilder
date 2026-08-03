# Universal Harness

`universal-harness/` turns any CLI tool, HTTP API, or CDP-debuggable GUI app into a
JSON **Capability Manifest** of named, typed operations that any agent (or a generated
MCP server) can call uniformly, regardless of what's underneath. It is explicitly a
**generic, portable, project-agnostic tool**, not something BrainBuilder-specific: per
its own README, it "has no dependency on the project it lives in. Copy it anywhere,
`npm install`, `npm run build`, done." BrainBuilder is its first real-world test
subject, materialized as `universal-harness/connectors/brainbuilder/`.

## Core architecture (`universal-harness/src/types.ts`)

A `CapabilityManifest` has a `target` (`CliTarget` / `ApiTarget` / `GuiTarget`,
discriminated by `kind`), a `generatedAt` timestamp, and an `operations[]` array of
`CliOperation | ApiOperation | GuiOperation`. GUI operations carry a
`selectorConfidence: 'stable' | 'best-effort'` flag — `stable` means a real
`data-testid`/`id`/`aria-label`/`name` was found; `best-effort` means a generated
nth-child CSS path (more fragile).

| Module | Purpose |
|---|---|
| `src/cli.ts` | The `harness` binary — flat subcommands `introspect-cli`, `introspect-api`, `introspect-gui`, `list`, `call`, `mcp` |
| `src/introspect/cli.ts` | Parses `--help` output, recurses one level into subcommands |
| `src/introspect/api.ts` | Reads an OpenAPI 3.x spec |
| `src/introspect/gui.ts` | Walks the live DOM over CDP to find clickable/fillable elements |
| `src/execute/*.ts` | One executor per target kind — `cli.ts` spawns subprocesses, `api.ts` makes HTTP calls, `gui.ts` performs CDP-driven clicks/fills/reads — unified by `executeOperation()` in `src/execute/index.ts`, which dispatches on `target.kind` |
| `src/cdp-client.ts` | A minimal Chrome DevTools Protocol client: fetches `/json/list`, opens a raw WebSocket, sends `Runtime.evaluate`. Works against any Chromium-based surface (Chrome/Edge, Electron, or Tauri's WebView2/WebKit CDP bridge). `connectCdp`'s `urlIncludes` option picks the correct page among multiple exposed windows — needed because Tauri/Electron apps often expose more than one CDP page target |
| `src/mcp/server.ts` | `createMcpServer()`/`buildToolRegistry()` turn one or more manifests into a real `@modelcontextprotocol/sdk` `Server`; each operation becomes a tool named `<manifest-name-slug>__<operation-name>` (namespaced to avoid collisions across merged manifests); `serveManifestsOverStdio()` runs it over stdio for any MCP client |

## The BrainBuilder connector (`universal-harness/connectors/brainbuilder/`)

The concrete integration point — per `.claude/skills/harness/SKILL.md`, "live-verified
end to end (no mocks)":

- **`pipeline-cli.manifest.json`** — introspected from `node ../scripts/ci/pipeline.mjs
  --help`; exposes one operation, `pipeline`, with params `full`/`package`/`install-hooks`
  mapping to that script's CLI flags. See
  [Scripts & Testing](scripts-and-testing.md#scriptscipipelinemjs).
- **`predict-api.manifest.json`** + **`predict-api.openapi.json`** — hand-authored
  OpenAPI for `gui/src-tauri/src/predict_server.rs`'s real route; exposes one
  operation, `predict` (`POST /predict`), against an ephemeral OS-assigned `baseUrl`
  (regenerated fresh each run — see the [Predict panel's local server](gui-panels.md#predict-panel)).
- **`gui.manifest.json`** — 75 real operations discovered live from BrainBuilder's
  running window (`cdpUrl: http://localhost:9222`, `urlIncludes: localhost:5173`), a
  mix of `stable` and `best-effort` selector confidence: `toggle-theme`, `graph-new-btn`,
  `graph-save-btn`, `undo`/`redo`, `validate-btn`, `export-train-btn`, `zoom-in`/`zoom-out`,
  `fit-view`, and more.
- **`live-predict-proof.mjs`** — a proof script using only universal-harness's own
  layers: `connectCdp` to invoke the real Tauri v1 IPC command `start_predict_server`
  (via `window.__TAURI_IPC__`), then `introspectApi`/`executeApi` against the real
  returned URL to call `predict` with an actual graph and dataset, verifying real
  prediction tensors come back, then `stop_predict_server`.

### A real bug this integration found and fixed

Until 2026-07-11, every Tauri IPC command failed "not found" during `tauri dev`
because WebView2/wry defaults every app to one shared profile directory
(`%LOCALAPPDATA%\EBWebView`) and all Tauri apps share the `http://tauri.localhost/`
origin — a service worker from a *different* local Tauri app was intercepting
BrainBuilder's page loads. Fixed in `gui/src-tauri/src/main.rs`'s `main()` by giving
BrainBuilder its own `WEBVIEW2_USER_DATA_FOLDER` (see
[GUI Architecture's startup section](gui-architecture.md#startup-main)).

## Usage

The workflow is introspect → list → call → mcp (see
`universal-harness/skills/harness/SKILL.md` for the canonical, generic skill doc —
`.claude/skills/harness/SKILL.md` is BrainBuilder's thin pointer to it). Typical
invocation to expose BrainBuilder as MCP tools:

```
node universal-harness/dist/src/cli.js mcp \
  --manifest .../pipeline-cli.manifest.json \
  --manifest .../gui.manifest.json \
  --name brainbuilder
```

This can itself be registered in `.mcp.json` at the repo root, so an agent can
recursively call BrainBuilder's own CI pipeline and live GUI as tools.

## Test coverage (`universal-harness/test/*.test.ts`)

Explicitly "no mocks" — every test drives something real:

- **`api.test.ts`** — `introspectApi`/`executeApi` against a real local HTTP fixture
  server, covering both success and error execution paths.
- **`cli.test.ts`** — `introspectCli` against a real fixture CLI script.
- **`cli-e2e.test.ts`** — the strongest test: spawns the compiled `harness` binary as a
  real separate OS process and drives it with a real MCP `Client`/`StdioClientTransport`
  — no in-process shortcuts.
- **`gui.test.ts`** — `introspectGui`/`executeGui` against a real headless Chromium
  browser (skipped if none found) loading a fixture HTML page.
- **`cdp-multi-target.test.ts`** — verifies `connectCdp`'s `urlIncludes` correctly
  picks between two simultaneously open page targets, mirroring BrainBuilder's
  multi-window scenario.
- **`mcp.test.ts`** — verifies a CLI manifest becomes real callable MCP tools via an
  in-memory MCP client/server pair.

Run via `npm test` (a `tsc` build followed by `node --test dist/test/**/*.test.js`).
