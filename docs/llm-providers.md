# LLM Providers

`core/src/llm/` abstracts LLM access so every authoring/synthesis path
([graph authoring](#graph-authoring-pipeline), [component synthesis](synthesis.md),
[the self-building agent](agent.md)) is provider-agnostic instead of hardcoding
Ollama.

## The `LlmProvider` trait (`core/src/llm/provider.rs`)

Four required methods:

- `id(&self) -> &str` — stable id, the left half of a `"provider:model"` selector.
- `display_name(&self) -> &str` — GUI label.
- `async fn list_models(&self) -> Result<Vec<String>>` — a live reachability probe for
  local providers, a static menu for hosted ones.
- `async fn generate_json(&self, model, system, prompt) -> Result<String>` — one
  non-streaming call, raw text returned. Callers parse and validate the result; a
  provider is never trusted structurally.

### `OllamaProvider`

Wraps `OllamaClient` (`core/src/llm/ollama.rs`), which talks loopback HTTP to
`http://localhost:11434`. `list_models` hits `/api/tags`; `generate_json` hits
`/api/generate` with `format:"json"`. Both fail fast with messages naming Ollama
specifically ("is it running? (`ollama serve`)", "is `{model}` pulled?") rather than
hanging or giving a generic error. This network hop can't be bound in an automated
sandbox — it's developer-verified only (see the
[Live Verification Checklist](VERIFICATION.md)).

### `OpenCodeProvider`

Hits OpenCode Go's hosted OpenAI-compatible `/chat/completions` endpoint (default base
URL `https://opencode.ai/zen/go/v1`). `list_models` returns a static 13-entry roster
namespaced `opencode-go/<model>` with no network or key needed. `generate_json`
requires a non-empty key or returns a `ConfigError` telling the user to add one in the
[Models panel](gui-panels.md#models-directory); it posts with `bearer_auth`,
`response_format: {"type":"json_object"}`, and surfaces HTTP-status/body and
JSON-shape failures with OpenCode named explicitly.

## `ProviderRegistry` (`core/src/llm/registry.rs`)

The single selector-resolution seam. `resolve(selector)` splits on the first `:`;
no-colon selectors default to `"ollama"` for backward compatibility. Resolving
`"opencode"` triggers the injected `opencode_key: Box<dyn Fn() -> Option<String>>`
closure — a keychain lookup happens **only** when an opencode selector is actually
resolved, so a user who never touches OpenCode never triggers a keychain read. Unknown
provider ids return a `ConfigError` naming the bad id and the known set. `available()`
returns the static `[("ollama",...), ("opencode",...)]` pair for the GUI's provider
dropdown — usability (is Ollama running? is a key set?) is a runtime concern surfaced
only when actually invoked.

## Credential storage (OS keychain)

`gui/src-tauri/src/main.rs` stores the OpenCode key in the OS-native secret store via
the `keyring` crate — never on disk, never in a saved graph.

- Constants: `KEYCHAIN_SERVICE = "brainbuilder"`, `OPENCODE_KEY_USER =
  "opencode-api-key"`.
- `read_opencode_key()` builds a `keyring::Entry::new(...)` and calls `.get_password()`,
  returning `None` (never an `Err`) on any failure so the provider layer can give a
  friendly "add your key" message instead of a hard crash.
- `provider_registry()` wires `read_opencode_key` into `ProviderRegistry::new`, so a key
  pasted at runtime takes effect on the very next call — no app restart.
- Tauri commands: `set_provider_credentials(provider, key)` (writes, or clears via
  `entry.delete_password()` on an empty key — treated as success even if nothing was
  stored), `has_provider_credentials(provider)` (reports presence without ever
  returning the secret to the frontend), `list_llm_providers`, `list_provider_models`.

`keyring` maps to OS-native stores: macOS Keychain, Windows Credential Manager, Linux
libsecret.

## Graph authoring pipeline (`core/src/llm/graph_author.rs`)

The main consumer of this layer. `build_system_prompt` enumerates the **live**
component registry (never a hardcoded list), so the model can only propose real
components and ports that actually exist. `generate_graph_with_provider` calls
`provider.generate_json` then `parse_and_validate`, which deserializes the result into
a `BBIRGraph` and runs it through the exact same `validate_graph` the canvas's
"Validate" button uses (see [Component System](component-system.md#validation-core-srccomponentvalidationrs))
— a hallucinated port or shape mismatch is caught here, and never handed to execution.

This is what powers the [LLM Author panel](gui-panels.md#llm--synthesis--agent-gui-side-only)
("Describe a Model") and the [Intent panel's](gui-panels.md#intent-panel) underlying
graph assembly.

## Local model loading (`core/src/models/`)

Related but distinct: this module is about **loading models already on disk**, not
authoring/generation. Per its module doc: `discovery.rs` finds models already cached
locally (`scan_local_models`, `default_hf_cache_dir`); `safetensors_loader.rs` loads
safetensors weights BrainBuilder parses itself; `onnx_loader.rs` (behind the `onnx`
feature) loads ONNX via the `ort` crate; `gguf_router.rs` is the actual touchpoint with
this LLM layer — `GgufRouter` delegates GGUF-format models to a real local `ollama`
install via `ollama create` (subprocess) and then calls generation through the same
`OllamaClient` used above, rather than reimplementing quantized-tensor inference. This
is model-*serving* plumbing for locally-run model weights, distinct from the
`llm::provider`/`registry` selector system used for graph authoring and component
synthesis, though it reuses `OllamaClient` as its HTTP transport. The GUI-side [Model
Hub](gui-panels.md#models-directory) is its frontend.

## Related

- [Component Synthesis](synthesis.md) — generating whole new components through this
  same provider layer.
- [Self-Building Agent](agent.md) — driving an external `opencode` coding agent, a
  related but separate integration.
- [Models directory (GUI)](gui-panels.md#models-directory) — where a user connects
  OpenCode and picks a provider/model.
