# universal-harness

Turn any CLI tool, HTTP API, or CDP-debuggable GUI app into a set of typed
operations an agent can discover and call — directly, or through a generated
[MCP](https://modelcontextprotocol.io) server any MCP-compatible agent can
connect to.

Portable by design: this folder has no dependency on the project it lives
in. Copy it anywhere, `npm install`, `npm run build`, done.

```
npm install
npm run build
npm test          # runs the full suite: real subprocesses, a real local
                   # HTTP server, and (if a Chromium-based browser is
                   # installed) a real headless-browser CDP round-trip
```

See [`skills/harness/SKILL.md`](skills/harness/SKILL.md) for the full
workflow (introspect → list → call → mcp) and concrete examples.

## Layout

```
src/
  types.ts              Capability Manifest schema — the one shape every layer agrees on
  cdp-client.ts          Minimal CDP client (fetch /json/list, raw WebSocket, Runtime.evaluate)
  introspect/
    cli.ts               --help parser -> operations
    api.ts                OpenAPI 3.x reader -> operations
    gui.ts                CDP DOM walker -> click/fill operations
  execute/
    cli.ts / api.ts / gui.ts    One executor per target kind
    index.ts              executeOperation() dispatches by manifest.target.kind
  mcp/
    server.ts             Turns manifest(s) into a real MCP server (stdio)
  cli.ts                  The `harness` binary tying it all together
skills/harness/SKILL.md   Agent-facing usage doc, travels with the folder
connectors/                Example: manifests generated against a real project (BrainBuilder)
test/                      Real subprocesses, a real local HTTP server, a real headless browser —
                            no mocks
```

## Design notes

- **Introspection is heuristic, not a guarantee.** CLI help text has no
  universal format; GUI selectors are ranked `stable` (a real `data-testid`/
  `id`/`aria-label`/`name` attribute) vs `best-effort` (a generated
  nth-child path) so a caller can judge how likely an operation is to break
  across UI changes. Manifests are plain JSON — hand-edit one if
  introspection got something wrong rather than fighting the heuristic.
- **Multi-window apps need explicit targeting.** Electron/Tauri apps
  routinely expose more than one page over the same CDP port (a main window
  plus a secondary popup, say). `connectCdp`'s `urlIncludes` option (and the
  CLI's `--url-includes`) picks the right one instead of guessing at
  whichever the browser lists first.
- **One dependency**: `@modelcontextprotocol/sdk`. Everything else (CLI
  spawning, HTTP, CDP, arg parsing) uses only Node's standard library, to
  keep "copy this folder anywhere and `npm install`" actually true.
