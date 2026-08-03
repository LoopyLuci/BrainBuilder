# OmniForge Desktop

Cross-platform agentic AI model development environment.
**Windows · Linux · (macOS later)**

Integrates the audited **OmniForge Concierge** with a React Flow canvas, model registry, and training controls.

## Windows 10 quick start

See **[WINDOWS.md](./WINDOWS.md)** for full instructions.

```powershell
npm install
npm run tauri dev
```

## Prerequisites

| Platform | Requirements |
|----------|--------------|
| **All** | Node.js ≥ 18, Rust (stable) |
| **Windows** | MSVC Build Tools, WebView2 |
| **Linux** | `libwebkit2gtk-4.0-dev build-essential libssl-dev libgtk-3-dev libayatana-appindicator3-dev` |
| **macOS** | Xcode CLT (enable target in `tauri.conf.json`) |

## Quick start

```bash
cd omniforge
npm install
npm run tauri dev      # hot-reload development
npm run tauri build    # production bundles
```

Artifacts appear under `src-tauri/target/release/bundle/`.

## Architecture

- **Frontend** – React + React Flow + Tailwind (Vite)
- **Backend** – Tauri 1.x + `concierge-core`
- **Agent** – ReAct loop, free OpenRouter models with fallback, SQLite memory
- **PlatformHost** – live canvas + model registry; emits `canvas-update` events
- **Sandbox** – bundled Python sandbox for generated code

## Using the Concierge

Open the right-hand panel and type natural language, e.g.:

- “Search for a small language model and add it to the canvas”
- “Create a LoRA training job on my dataset”
- “Merge model-a and model-b with linear strategy”

The agent calls platform tools; the canvas updates in real time.

## Environment

| Variable | Purpose |
|----------|---------|
| `OPENROUTER_API_KEY` | Optional; free models work without it |
| `CONCIERGE_SANDBOX` | Path to `sandbox.py` (defaults to sibling folder) |
| `RUST_LOG` | `info` / `debug` |

## License

MIT


## Persistence

Canvas nodes/edges and the model registry are stored in **SQLite** (`omniforge.db`).
They survive restarts and are reloaded automatically on launch.

## Model execution

| Format | Backend | Enable |
|--------|---------|--------|
| **ONNX** | ONNX Runtime (`ort` crate) | `cargo build --features onnx` |
| **GGUF** | `llama-server` on PATH | Always available |

### Frontend examples

```ts
// Load ONNX model
const sessionId = await invoke("load_onnx_model", { path: "/path/to/model.onnx" });

// Run inference (flat f32 arrays)
const result = await invoke("infer_onnx", {
  sessionId,
  inputs: {
    input_ids: [1, 2, 3, 4],
    "__shapes__": { input_ids: [1, 4] }
  }
});

// GGUF
const procId = await invoke("start_gguf_model", { path: "/path/to/model.gguf", port: 8080 });
```

Build with ONNX support:

```bash
cd src-tauri
cargo build --features onnx
# or
npm run tauri build -- --features onnx
```


## SQLite performance

OmniForge opens the database with:

| PRAGMA | Value | Effect |
|--------|-------|--------|
| `journal_mode` | WAL | Concurrent readers + fast writers |
| `synchronous` | NORMAL | Safe with WAL, far faster than FULL |
| `cache_size` | 64 MB | Fewer disk hits |
| `temp_store` | MEMORY | Temp tables stay in RAM |
| `busy_timeout` | 5 s | Avoids immediate lock errors |
| `mmap_size` | 256 MB | Memory-mapped I/O |

Bulk canvas restores use a single transaction via `persist_canvas_batch`.
Indexes exist on `models(name)`, `models(format)`, and edge source/target.

## GGUF support

- Probes `llama-server`, `llama-cpp-server`, and `server` on PATH
- Validates `.gguf` extension and minimum file size
- Optional GPU offload: `OMNIFORGE_N_GPU_LAYERS`
- Context size: `OMNIFORGE_CTX_SIZE` (default 2048)
- Health-checks `http://127.0.0.1:{port}/health` after spawn
- Tracks process status; dead processes are pruned from the list

```ts
const id = await invoke("start_gguf_model", { path: "/models/phi.gguf", port: 8080 });
const list = await invoke("list_gguf_models"); // [{ id, model_path, port, endpoint, status }]
```

## Error handling

ONNX and GGUF errors are structured (`ExecError`) and surface as clear strings:

- `FileNotFound` / `InvalidFormat`
- `BinaryNotFound` (lists binaries tried)
- `ProcessSpawn` / `ProcessExited` (includes stderr snippet)
- `HealthCheckFailed`
- `OnnxNotCompiled` / `OnnxInit` / `OnnxLoad` / `OnnxInfer`
- `InvalidInput` / `SessionNotFound`


## Atomic filesystem

All self-edits and critical config writes use `atomic_fs`:

1. Create temp file **in the same directory** as the target (avoids cross-device `EXDEV`)
2. Write payload + `sync_all` on the temp file
3. `rename` over the target (atomic on POSIX; replace fallback on Windows)
4. `fsync` parent directory on Unix for durable directory entries
5. `cleanup_orphans` removes `*.omniforge-tmp*` after crashes

Also available: `atomic_copy`, `atomic_cas` (compare-and-swap), `durable_append_line`.

## Tauri plugin architecture

Rust capabilities register as `OmniPlugin` in `src-tauri/src/plugins/`:

| Plugin | Role |
|--------|------|
| `omniforge-fs` | Atomic write / copy / CAS / orphan cleanup |

```rust
let mut reg = PluginRegistry::new();
reg.register(Arc::new(FsPlugin));
reg.setup_all(&PluginContext { app: Some(handle) })?;
```

Future split: publish as `tauri-plugin-omniforge-fs` using `tauri::plugin::PluginBuilder`.
Python community plugins remain under `plugins/*/manifest.json`.
