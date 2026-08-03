# OmniForge – Complete Build Context for Agents

This tree is a **full** Tauri 1.x + React + `concierge-core` desktop app.
There are **no** `todo!()`, `unimplemented!()`, or intentional stubs in the agent path.

## Layout

```
omniforge/
├── package.json, vite.config.ts, tailwind, tsconfig*   # Frontend toolchain
├── index.html, src/                                    # React UI
├── concierge-core/                                     # Rust agent library (path dep)
├── concierge-python-sandbox/                           # Restricted Python runner
├── python/                                             # train_lora.py, augment_dataset.py
├── plugins/example_plugin/                             # Sample community plugin
├── scripts/                                            # Windows setup/run
├── src-tauri/                                          # Tauri backend
│   ├── Cargo.toml                                      # depends on ../concierge-core
│   ├── tauri.conf.json                                 # msi/nsis/deb/appimage
│   ├── icons/                                          # Valid ICO/PNG
│   └── src/
│       ├── main.rs                     # Boots host + ConciergeAgent + commands
│       ├── concierge_bridge.rs         # All Tauri invoke handlers
│       ├── platform_host.rs            # PlatformHost → SQLite canvas/models
│       ├── model_executor.rs           # GGUF + ONNX (feature-gated)
│       ├── inference_sandbox.rs        # Streaming graph execution
│       ├── training_executor.rs        # LoRA jobs + structured errors
│       ├── km_format.rs, export_bundle.rs, multimodal_merger.rs
│       ├── rag.rs, vector_store.rs     # Hybrid BM25 + dense retrieval
│       ├── atomic_fs.rs, self_edit.rs, paths.rs
│       ├── plugin_system.rs, plugins/  # Python + Rust plugin layers
│       └── …
└── WINDOWS.md, README.md, AGENT_BUILD.md
```

## Integration contract (must not break)

1. `concierge_core::PlatformHost` has 16 async methods (see `tools/implementations.rs`).
2. `main.rs` defines `SharedHost(Arc<OmniForgeHost>)` implementing **all 16**.
3. `ConciergeAgent::with_host(llm, sqlite_url, Box::new(SharedHost(...)))`.
4. Frontend `invoke("concierge_chat", { message })` → agent tools → same `OmniForgeHost` as UI commands.
5. Canvas sync: host emits `canvas-update`; React listens in `Canvas.tsx`.

## Build (Windows 10)

```powershell
# Prereqs: Node ≥18, Rust stable, VS Build Tools C++, WebView2
cd omniforge
npm install
npm run tauri dev          # development
npm run tauri build        # MSI/NSIS under src-tauri/target/release/bundle/
```

Optional: `OPENROUTER_API_KEY`, `OMNIFORGE_DATA_DIR`, `OMNIFORGE_PYTHON`, `RUST_LOG=info`.

ONNX: `npm run tauri build -- --features onnx` (needs ORT shared lib).

## Data dirs (Windows)

- `%APPDATA%\OmniForge\omniforge.db` – canvas + models
- `%APPDATA%\OmniForge\concierge.db` – agent memory

## Key entry points

| Concern | File |
|---------|------|
| Agent loop | `concierge-core/src/agent.rs` |
| Tool schemas | `concierge-core/src/tools/schemas.rs` |
| LLM / free models | `concierge-core/src/openrouter.rs` |
| Host persistence | `src-tauri/src/platform_host.rs` |
| Commands | `src-tauri/src/concierge_bridge.rs` |
| UI shell | `src/App.tsx` |
| Chat | `src/components/ConciergePanel.tsx` + `hooks/useConcierge.ts` |

## Quality gates before claiming “done”

- [ ] `cargo check` in `src-tauri` (and `concierge-core`)
- [ ] `npm run build` (frontend)
- [ ] `npm run tauri dev` opens window; Concierge replies; adding a node updates canvas
- [ ] PlatformHost method parity: trait ⊆ SharedHost ⊆ OmniForgeHost
