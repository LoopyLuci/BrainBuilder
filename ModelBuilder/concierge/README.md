# OmniForge Concierge

**Next-generation agentic personal assistant** that controls the entire OmniForge platform through natural language.

- **Correct ReAct agent loop** – single assistant message when both content and tool_calls are present
- **SQLite memory** that persists `tool_call_id` and `tool_calls` so multi-turn tool conversations survive restarts
- **Generic LLM client** – OpenRouter (free models), local Ollama, any OpenAI-compatible API
- **OpenCode Go / Zen style** local code generation with task-aware scaffolds (training, data, plugins, merge, inference)
- **Mandatory restricted Python sandbox** – no unsafe fallback
- **Graceful tool error recovery** – tool failures become `ToolResult::err` and are fed back to the model
- **Dynamic model switching** via `set_model` tool / `set_model()` API
- **Complete tool surface** for model import, canvas manipulation, training, knowledge modules, merging, docs, plugins
- Production-ready Rust (`concierge-core`) + Python sandbox – no stubs, no `todo!()`

Personal use and individual power-users are first-class citizens. The same library integrates cleanly into the OmniForge Tauri desktop application.

---

## Architecture

```
concierge/
├── concierge-core/                 # Rust library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs
│   │   ├── agent.rs                # ReAct loop (fixed single-message semantics)
│   │   ├── llm_client.rs
│   │   ├── openrouter.rs
│   │   ├── code_agent.rs           # Task-aware local generator + mandatory sandbox
│   │   ├── memory.rs               # tool_call_id + tool_calls persisted
│   │   ├── error.rs
│   │   └── tools/
│   │       ├── mod.rs
│   │       ├── schemas.rs
│   │       ├── registry.rs
│   │       └── implementations.rs
│   └── tests/
│       └── agent_loop.rs
├── concierge-python-sandbox/
│   ├── sandbox.py                  # Hardened AST + import restrictions
│   └── requirements.txt
└── README.md
```

---

## Quick start

```rust
use concierge_core::{ConciergeAgent, OpenRouterClient, FREE_MODELS};
use std::sync::Arc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Arc::new(OpenRouterClient::new(None)); // free tier
    // Or: OpenRouterClient::ollama("http://localhost:11434")

    let mut agent = ConciergeAgent::new(client, "sqlite://concierge.db")
        .await?
        .with_model(FREE_MODELS[0]);

    let reply = agent
        .chat("Search for a small language model and add it to the canvas at (100, 200)")
        .await?;
    println!("{}", reply);
    Ok(())
}
```

### Custom platform host

```rust
struct RealHost { /* … */ }

#[async_trait]
impl PlatformHost for RealHost {
    // implement every method with real canvas / registry calls
}

let agent = ConciergeAgent::with_host(client, "sqlite://…", Box::new(RealHost { … })).await?;
```

### Tauri command

```rust
#[tauri::command]
async fn concierge_chat(
    state: tauri::State<'_, AppState>,
    message: String,
) -> Result<String, String> {
    let mut agent = state.concierge.lock().await;
    agent.chat(&message).await.map_err(|e| e.to_string())
}
```

---

## Audit fixes applied

| Issue | Resolution |
|-------|------------|
| Duplicate assistant messages | Single message carrying both `content` and `tool_calls` |
| Missing `tool_call_id` in memory | Schema extended; history rebuild attaches ids correctly |
| OpenCode placeholder | Task-aware local scaffolds (train / data / plugin / merge / infer) |
| Unsafe sandbox fallback | Removed – missing sandbox now fails hard |
| Tool errors crash agent | Converted to `ToolResult::err` and fed back to the model |
| Hard-coded model only | `FREE_MODELS`, `with_model` / `set_model` / `set_model` tool |
| Weak sandbox | AST walker blocks dunder escapes + expanded deny lists |

---

## Tools

| Tool | Purpose |
|------|---------|
| `search_models` | Query local model registry |
| `import_model` | Import GGUF / ONNX / Safetensors / … |
| `list_models` | List registered models |
| `add_node` / `connect_nodes` / `modify_graph` | Canvas ops |
| `inspect_node` / `execute_graph` / `compare_outputs` | Runtime |
| `create_dataset` / `run_training` | Data & training |
| `create_knowledge_module` / `merge_models` | KM & merges |
| `search_docs` / `write_plugin` | Docs & plugins |
| `get_platform_status` | Health |
| `set_code_agent_mode` | OpenCode Go ↔ Zen |
| `execute_python` | Generate / run sandboxed Python |
| `set_model` | Switch active LLM |

---

## Configuration

| Env var | Meaning |
|---------|---------|
| `CONCIERGE_SANDBOX` | Absolute path to `sandbox.py` (required for code execution) |
| `RUST_LOG` / `RUST_LOG=concierge_core=debug` | Tracing |

---

## Tests

```bash
cd concierge-core
cargo test --test agent_loop
```

---

## License

MIT
