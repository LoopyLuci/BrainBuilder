---
name: harness
description: Turn any CLI tool, HTTP API, or CDP-debuggable GUI app (Electron, Tauri, Chrome/Edge) into a set of typed operations you can call directly or expose to any MCP client. Use when asked to "harness", "wrap", or "give agents access to" a piece of software that has no existing MCP server.
---

# Universal Harness

This is a portable, dependency-light toolkit (`universal-harness/`) that turns
three kinds of software into the same thing — a **Capability Manifest**
(JSON) of named operations — so any agent can discover and call them the same
way regardless of what's underneath:

- **CLI tools** — introspected by running `--help` (and one level into
  subcommands) and parsing flags/positional args heuristically.
- **HTTP APIs** — introspected from an OpenAPI 3.x JSON spec (local file or
  URL).
- **GUI apps** — introspected by connecting to a CDP (Chrome DevTools
  Protocol) debug port and walking the live DOM for interactive elements.
  Works on anything Chromium-based: real Chrome/Edge, Electron apps, and
  Tauri apps using the WebView2/WebKit CDP bridge.

This folder is meant to be copied wholesale into any project. It has no
dependency on the project it's copied into — only `npm install` inside
`universal-harness/` itself.

## Setup (once per copy of this folder)

```
cd universal-harness
npm install
npm run build       # compiles to dist/, also builds harness's `bin`
```

The compiled binary is `dist/src/cli.js` (`node dist/src/cli.js ...`, or
`npm link` to get a real `harness` command on PATH).

## Workflow

### 1. Introspect the target → get a manifest

```
# CLI tool
node dist/src/cli.js introspect-cli --command "git" --max-depth 1 --out git.manifest.json

# HTTP API (needs an OpenAPI 3.x JSON spec, local or URL)
node dist/src/cli.js introspect-api --spec ./openapi.json --base-url http://localhost:8080 --out api.manifest.json

# GUI app — the target must already be running with CDP debugging enabled
# (e.g. `--remote-debugging-port=9222`, or a Tauri dev build wired the same way)
node dist/src/cli.js introspect-gui --cdp-url http://localhost:9222 --out gui.manifest.json
```

Every introspector is **heuristic, not a guarantee**. Always inspect the
manifest (or run `list`, below) before trusting it — especially for CLI help
text, which has no universal format, and GUI selectors, which are marked
`"selectorConfidence": "best-effort"` when no stable attribute
(`data-testid`, `data-tutorial`, `id`, `aria-label`, `name`) was found on the
element. A `best-effort` selector is a generated nth-child CSS path and is
more likely to break across UI changes than a `stable` one.

### 2. Inspect what was found

```
node dist/src/cli.js list --manifest git.manifest.json
```

### 3. Call an operation directly (no MCP needed for quick checks)

```
node dist/src/cli.js call --manifest git.manifest.json --op status --args '{}'
```

Returns `{ ok, output, error, raw }` as JSON on stdout, exit code mirrors
`ok`.

### 4. Expose it to any MCP client (the real point of this toolkit)

```
node dist/src/cli.js mcp --manifest git.manifest.json --manifest api.manifest.json --name my-tools
```

This starts a standard MCP server over stdio. Any MCP-compatible agent
(Claude Code, Claude Desktop, or any other agent framework speaking MCP) can
connect and see every operation from every manifest passed, namespaced as
`<manifest-name-slug>__<operation-name>` to avoid collisions when merging
multiple targets into one server. Register it the normal way a client
registers any stdio MCP server, e.g. in `.mcp.json`:

```json
{
  "mcpServers": {
    "my-tools": {
      "command": "node",
      "args": ["/absolute/path/to/universal-harness/dist/src/cli.js", "mcp", "--manifest", "/absolute/path/to/git.manifest.json"]
    }
  }
}
```

## When to use which layer

- Software has a `--help` you can read → **CLI** introspection. Fastest,
  most reliable, works headless.
- Software has (or you can hand-author) an OpenAPI spec → **API**
  introspection. Also fast and reliable; no UI needed.
- Neither of the above, but it's a real window you can look at (a desktop
  app with no API, a legacy internal tool) → **GUI** introspection. Requires
  the target to be Chromium-based and reachable via CDP. Slower and more
  fragile than the other two — prefer CLI/API when either is available.

## Extending

- `src/introspect/*.ts` + `src/execute/*.ts` are independent pairs — add a
  new target kind by adding both files and a new `Target`/`Operation`
  variant in `src/types.ts`.
- Manifests are plain JSON and hand-editable — if introspection gets a
  parameter or selector wrong, just fix the manifest file directly rather
  than fighting the heuristic.
