#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod bot_dashboard;
mod cluster_actor;
mod observer_server;
mod predict_server;
mod model_catalog;
mod multimodal_merger;
mod model_executor;
mod causal_reasoning;
mod graph_of_thoughts;
mod concept_bottleneck;
mod mixture_of_experts;
mod neural_architecture_search;
mod differentiable_neural_computer;
mod hyperdimensional_computing;
mod spiking_neural_network;
mod world_model;
mod program_synthesis;
mod multimodal_alignment;
mod next_gen_attention;
mod retrieval_augmented_generation;
mod adaptive_reasoning;
mod knowledge_graph;
mod flow_analyzer;
mod counterfactual_explainer;
mod continual_learning;
mod symbolic_reasoning;
mod temporal_point_process;
mod interactive_explainability;
mod compositional_reasoning;
mod neuro_symbolic_prover;
mod federated_learning;
mod pmi_analyzer;
mod uncertainty_quantification;
mod hyperparameter_optimizer;
mod power_manager;
mod compression_agent;
mod self_improving_commands;
mod meta_model_builder;
mod telemetry;
mod active_learning_loop;
mod online_learning_loop;
mod fact_checking_model;
mod data_assistant_model;
mod scraping_model;
mod sandbox_executor;
mod meta_controller;
mod memory_recall;
mod safety_harness;
mod eval_harness;
mod model_mistress;
mod luci;
mod luci_store;
mod nervous_system;
pub mod tool_executor;
mod webview_debug;

use tauri::{command, Manager, State};
use brainbuilder_core::orchestrator::Orchestrator;
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::data::metrics::subscribe_metrics;
use cluster_actor::{ClusterHandle, ClusterStatus};
use power_manager::{pm_health, pm_list_domains, pm_apply_power_limit, pm_recent_telemetry, pm_list_blueprints, pm_build_mpc_spec};
use compression_agent::{ca_health, ca_compress, ca_recent_stats, ca_list_blueprints, ca_build_compressor_spec};
use self_improving_commands::{
  luci_status, luci_greet, luci_chat, luci_propose_plan, luci_list_plans, luci_update_plan_status, luci_reflect, luci_recent_reflections, luci_set_preference, luci_get_preference, luci_remember_fact, luci_recall_memories, luci_forget_memory, luci_audit, luci_recent_audit, luci_register_tool, luci_improve,
};
use nervous_system::commands::{nervous_system_status, nervous_system_providers, nervous_system_submit, nervous_system_start, nervous_system_stop, nervous_system_list};
use tool_executor::registry::ToolRegistry;
use tool_executor::commands::{tool_executor_run, tool_executor_list};
use bot_dashboard::{bot_dashboard_status, bot_dashboard_settings_get, bot_dashboard_settings_set, bot_dashboard_start, bot_dashboard_stop, bot_dashboard_restart, bot_dashboard_events, bot_dashboard_clear_events, bot_dashboard_telemetry, bot_autostart};
use webview_debug::{webview_debug_eval, webview_debug_query, webview_debug_click, webview_debug_fill, webview_debug_snapshot, webview_debug_get_state, webview_debug_set_enabled, webview_debug_is_enabled};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, OnceCell};

struct AppState {
    // Orchestrator's own registry has interior mutability (RwLock), so this
    // outer Mutex only needs to serialize access to the Orchestrator struct
    // itself (trainer_controller, python bridge), not gate every read.
    orchestrator: Mutex<Orchestrator>,
    // Filled in once the Cluster actor finishes its (async) startup — see
    // `setup_cluster` below. Reads/writes to the cell itself never block on
    // the swarm, so this doesn't slow down any of the ML-training commands.
    cluster: Arc<OnceCell<ClusterHandle>>,
    // Filled in once the observer HTTP server (Phase 5's phone/tablet view)
    // has bound a real port and detected this machine's LAN address.
    observer_url: Arc<OnceCell<String>>,
    // Where components live on disk — needed by component synthesis to write a
    // newly-synthesized descriptor + kernel so `install_component` can register
    // it live.
    components_dir: std::path::PathBuf,
    // The active self-building agent session (isolated git worktree + branch),
    // if one is running. At most one at a time keeps the safety model simple.
    agent: Mutex<Option<brainbuilder_core::agent::AgentSession>>,
    // The running local predict HTTP server (see `predict_server.rs`), if
    // one has been started: its URL plus the sender that shuts it down. At
    // most one at a time, same reasoning as `agent` above.
    predict_server: Mutex<Option<(String, tokio::sync::oneshot::Sender<()>)>>,
    // Bot dashboard state
    bot: Arc<Mutex<bot_dashboard::BotServerHandle>>,
    // WebView2 debugging / agent control state
    debug: Arc<Mutex<webview_debug::DebugState>>,
}

async fn cluster_handle(state: &State<'_, AppState>) -> Result<ClusterHandle, String> {
    // The actor starts almost immediately (keypair load/generate + a local
    // TCP bind), so a real caller only ever hits this loop on the very first
    // command issued right after app launch.
    for _ in 0..50 {
        if let Some(handle) = state.cluster.get() {
            return Ok(handle.clone());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("cluster subsystem did not start in time".to_string())
}

#[command]
async fn get_cluster_status(state: State<'_, AppState>) -> Result<ClusterStatus, String> {
    Ok(cluster_handle(&state).await?.status().await)
}

#[command]
async fn create_cluster(display_name: String, state: State<'_, AppState>) -> Result<ClusterStatus, String> {
    Ok(cluster_handle(&state).await?.create_cluster(display_name).await)
}

#[command]
async fn generate_pairing_code(state: State<'_, AppState>) -> Result<String, String> {
    cluster_handle(&state).await?.generate_pairing_code().await
}

#[command]
async fn join_cluster_with_code(
    code: String,
    display_name: String,
    state: State<'_, AppState>,
) -> Result<ClusterStatus, String> {
    let handle = cluster_handle(&state).await?;
    tokio::time::timeout(Duration::from_secs(10), handle.join_with_code(code, display_name))
        .await
        .map_err(|_| "no device on this network accepted that pairing code".to_string())?
}

/// Compiles the current canvas graph and starts hosting it as a
/// data-parallel training job on the Cluster: `expected_clients` other
/// paired devices must join before the first gradient-averaging round runs.
/// Live loss is published through the same `metrics-update` event the local
/// trainer uses, so the Metrics tab shows distributed progress with no
/// separate UI path. Returns the new job's id.
#[command]
async fn host_distributed_job(
    graph_json: String,
    expected_clients: usize,
    state: State<'_, AppState>,
) -> Result<String, String> {
    cluster_handle(&state).await?.host_training(graph_json, expected_clients).await
}

/// Jobs this device has heard gossiped on the Cluster (from any paired
/// device, including a job it's hosting itself) and can join.
#[command]
async fn list_distributed_jobs(state: State<'_, AppState>) -> Result<Vec<cluster_actor::JobInfo>, String> {
    Ok(cluster_handle(&state).await?.list_jobs().await)
}

/// Joins a gossiped job as a compute contributor: trains on this device's
/// own locally-configured dataset, submitting real gradients each round and
/// receiving the host's averaged weights back.
#[command]
async fn join_distributed_job(job_id: String, state: State<'_, AppState>) -> Result<(), String> {
    cluster_handle(&state).await?.join_training(job_id).await
}

/// This device's own hosted-or-joined training job progress, if any.
#[command]
async fn get_distributed_training_status(
    state: State<'_, AppState>,
) -> Result<Option<cluster_actor::TrainingStatus>, String> {
    Ok(cluster_handle(&state).await?.training_status().await)
}

/// A phone/tablet on the same LAN opens this URL to see the Cluster's live
/// status (`observer_server.rs`) — no app install, no mobile toolchain, real
/// data over a real HTTP server, matching the "any device" personal-first
/// goal for the device classes that can't run a full Node.
#[command]
async fn get_observer_url(state: State<'_, AppState>) -> Result<String, String> {
    for _ in 0..50 {
        if let Some(url) = state.observer_url.get() {
            return Ok(url.clone());
        }
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    Err("observer server did not start in time".to_string())
}

/// Structural + shape check only — no training, no data loading, no Python
/// worker call. Lets the GUI surface shape-inference errors (see
/// `component::validation::validate_graph`) the moment a graph is edited,
/// well before the (much slower, much later) `execute_graph` would hit them.
#[command]
async fn validate_graph(graph_json: String, state: State<'_, AppState>) -> Result<(), String> {
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.validate(&graph).map_err(|e| e.to_string())
}

/// Local-first LLM-assisted authoring: turns a plain-English description
/// into a real BBIR graph via a local Ollama model, validated against the
/// live component registry before ever reaching the canvas. Returns the
/// graph as a JSON string (matching `load_graph`'s convention) rather than
/// a native `BBIRGraph` — Tauri's IPC layer round-trips through JSON either
/// way, and this keeps the same shape the frontend already knows how to
/// consume via `convertFromBBIR`.
/// OS keychain coordinates for the OpenCode API key. `keyring` maps these to
/// the platform-native secret store (macOS Keychain, Windows Credential
/// Manager, Linux libsecret) — the key is never written to disk or the graph.
const KEYCHAIN_SERVICE: &str = "brainbuilder";
const OPENCODE_KEY_USER: &str = "opencode-api-key";

/// Reads the stored OpenCode key from the OS keychain, if any. Returns `None`
/// (never an error) when unset so the provider layer can give a friendly
/// "add your key" message rather than a keychain-plumbing error.
fn read_opencode_key() -> Option<String> {
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, OPENCODE_KEY_USER).ok()?;
    entry.get_password().ok()
}

/// Builds a `ProviderRegistry` whose OpenCode key is sourced live from the OS
/// keychain — so a key pasted into the Models panel takes effect on the next
/// authoring/synthesis call with no restart.
fn provider_registry() -> brainbuilder_core::llm::ProviderRegistry {
    brainbuilder_core::llm::ProviderRegistry::new(Box::new(read_opencode_key))
}

/// The LLM providers the GUI selector should offer, as `[id, display_name]`.
#[command]
async fn list_llm_providers() -> Result<Vec<[String; 2]>, String> {
    Ok(provider_registry()
        .available()
        .into_iter()
        .map(|(id, name)| [id, name])
        .collect())
}

/// Every GPU wgpu can drive on this machine (Vulkan/DX12/Metal/GL), for the
/// device picker. Empty on a machine with no compatible GPU.
#[command]
async fn list_gpu_adapters() -> Result<Vec<brainbuilder_core::runtime::wgpu_backend::GpuAdapterInfo>, String> {
    // Enumeration touches the GPU driver; keep it off the async reactor.
    tokio::task::spawn_blocking(brainbuilder_core::runtime::wgpu_backend::list_adapters)
        .await
        .map_err(|e| e.to_string())
}

/// Bind a specific GPU by name (substring, e.g. "7900 XTX") and report the
/// adapter actually acquired — the live "does my card work?" probe. Returns the
/// bound adapter's real name so the UI can confirm the intended device.
#[command]
async fn probe_gpu_adapter(name: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || {
        let want = if name.trim().is_empty() { None } else { Some(name.as_str()) };
        brainbuilder_core::runtime::wgpu_backend::WgpuDevice::with_preferred(want)
            .map(|d| d.adapter_name().to_string())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

/// Set the preferred GPU (by adapter-name substring) that native `rust` ops run
/// on. An empty string clears it (auto/CPU). Persisted app-side; consumed by the
/// orchestrator when running a graph forward.
#[command]
async fn set_preferred_gpu(name: String, state: State<'_, AppState>) -> Result<(), String> {
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.set_preferred_gpu(if name.trim().is_empty() { None } else { Some(name) });
    Ok(())
}

/// The models a given provider can serve right now — a live reachability probe
/// for Ollama, the published roster for OpenCode.
#[command]
async fn list_provider_models(provider: String) -> Result<Vec<String>, String> {
    // Resolve with an empty model; we only need the provider handle to list.
    let (handle, _) = provider_registry().resolve(&format!("{provider}:")).map_err(|e| e.to_string())?;
    handle.list_models().await.map_err(|e| e.to_string())
}

/// Stores (or clears, when `key` is empty) the OpenCode API key in the OS
/// keychain. Only the `opencode` provider currently has credentials.
#[command]
async fn set_provider_credentials(provider: String, key: String) -> Result<(), String> {
    if provider != "opencode" {
        return Err(format!("provider `{provider}` has no credentials to set"));
    }
    let entry = keyring::Entry::new(KEYCHAIN_SERVICE, OPENCODE_KEY_USER).map_err(|e| e.to_string())?;
    if key.trim().is_empty() {
        // `delete_password` errors if nothing was stored — that's fine, treat
        // "clear an unset key" as success.
        let _ = entry.delete_password();
        Ok(())
    } else {
        entry.set_password(&key).map_err(|e| e.to_string())
    }
}

/// Whether a provider's credentials are present (so the GUI can show
/// "connected" without ever reading the secret back into the frontend).
#[command]
async fn has_provider_credentials(provider: String) -> Result<bool, String> {
    Ok(provider == "opencode" && read_opencode_key().is_some())
}

#[command]
async fn generate_graph(description: String, selector: String, state: State<'_, AppState>) -> Result<String, String> {
    // The registry lock is `std::sync::RwLock` (fine for every other command
    // here, which only ever reads it synchronously) — its guard isn't
    // `Send`, so it can't be held across the network `.await` below. Build
    // the prompt and drop the guard first, make the (only) async call with
    // no lock held, then re-acquire briefly to validate the result.
    let system = {
        let orchestrator = state.orchestrator.lock().await;
        let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
        brainbuilder_core::llm::build_system_prompt(&registry.summaries())
    };

    // Resolve the `"provider:model"` selector to a concrete provider (Ollama or
    // OpenCode); a bare model name still defaults to Ollama for back-compat.
    let (provider, model) = provider_registry().resolve(&selector).map_err(|e| e.to_string())?;
    let raw = provider.generate_json(&model, &system, &description).await.map_err(|e| e.to_string())?;

    let orchestrator = state.orchestrator.lock().await;
    let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
    let graph = brainbuilder_core::llm::parse_and_validate(&raw, &registry).map_err(|e| e.to_string())?;
    serde_json::to_string(&graph).map_err(|e| e.to_string())
}

/// Component synthesis: turn a description into a brand-new component
/// (descriptor + kernel), run the *static* half of the validation gauntlet
/// (parse + structure + name-collision), then run the **sandboxed smoke test**
/// and return everything — including the smoke result — WITHOUT installing.
/// The frontend shows the descriptor/kernel/smoke result and only calls
/// `install_synthesized_component` on user acceptance.
#[command]
async fn synthesize_component(description: String, selector: String, state: State<'_, AppState>) -> Result<String, String> {
    // Same non-`Send` registry-lock dance as generate_graph: build the prompt
    // under the lock, drop it before the network await, re-acquire to validate.
    let existing = {
        let orchestrator = state.orchestrator.lock().await;
        let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
        registry.list_names()
    };
    let system = brainbuilder_core::synthesis::build_synthesis_prompt(&existing);

    let (provider, model) = provider_registry().resolve(&selector).map_err(|e| e.to_string())?;

    // Self-repair loop: on a validation or smoke-test failure, feed the model
    // its own output + the exact error and let it try once more before we
    // surface a failure. Most first-shot misses (a shape mismatch, a missing
    // entry fn) are mechanically fixable, so this markedly raises the success
    // rate for a non-expert without ever relaxing the gauntlet.
    const MAX_ATTEMPTS: usize = 2;
    let mut user_msg = description.clone();
    let mut last_error = String::from("synthesis produced no result");
    for attempt in 0..MAX_ATTEMPTS {
        let is_last = attempt + 1 == MAX_ATTEMPTS;
        let raw = provider.generate_json(&model, &system, &user_msg).await.map_err(|e| e.to_string())?;

        // Static gates (parse + structure) under the registry lock; drop it
        // before any await so the non-`Send` guard never crosses one.
        let parsed = {
            let orchestrator = state.orchestrator.lock().await;
            let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
            brainbuilder_core::synthesis::parse_synthesis_output(&raw, &registry)
        };
        let component = match parsed {
            Ok(c) => c,
            Err(e) => {
                last_error = e.to_string();
                if is_last {
                    return Err(format!("synthesis failed after {MAX_ATTEMPTS} attempts: {last_error}"));
                }
                user_msg = brainbuilder_core::synthesis::build_repair_request(&description, &raw, &last_error);
                continue;
            }
        };

        // Third gate: run the kernel in the nervous-system sandbox on tiny tensors.
        let smoke = brainbuilder_core::synthesis::run_smoke_test(&component).map_err(|e| e.to_string())?;

        if smoke.passed || is_last {
            // Return the serializable artifacts + smoke report; the descriptor
            // is re-parsed on install, so nothing untrusted is trusted across
            // the hop. `attempts` lets the UI note a successful self-repair.
            let payload = serde_json::json!({
                "name": component.name,
                "descriptor_edn": component.descriptor_edn,
                "python_code": component.python_code,
                // Echo the exact smoke-test shapes so the install call can
                // round-trip them back verbatim (parse_synthesis_output
                // requires them to line up with the descriptor's input ports).
                "smoke_test": component.smoke_test,
                "smoke": smoke,
                "attempts": attempt + 1,
            });
            return serde_json::to_string(&payload).map_err(|e| e.to_string());
        }

        // Smoke failed with an attempt left — repair and retry.
        last_error = smoke.detail.clone();
        user_msg = brainbuilder_core::synthesis::build_repair_request(&description, &raw, &last_error);
    }

    Err(format!("synthesis failed after {MAX_ATTEMPTS} attempts: {last_error}"))
}

/// Install a previously-synthesized component. Never trusts the round-trip:
/// re-parses + re-validates the descriptor against the live registry, re-runs
/// the sandboxed smoke test, and only on green writes it to disk and
/// hot-registers it so it appears in the palette live. `component_json` is the
/// object `synthesize_component` returned, plus its `smoke_test` shapes.
#[command]
async fn install_synthesized_component(component_json: String, state: State<'_, AppState>) -> Result<(), String> {
    // Re-run the full static gauntlet against the current registry (rejects a
    // name taken since synthesis, a descriptor that no longer parses, etc.).
    let component = {
        let orchestrator = state.orchestrator.lock().await;
        let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
        brainbuilder_core::synthesis::parse_synthesis_output(&component_json, &registry).map_err(|e| e.to_string())?
    };

    // Re-run the sandboxed smoke test; refuse to install anything that doesn't
    // actually run and produce its promised shape.
    let smoke = brainbuilder_core::synthesis::run_smoke_test(&component).map_err(|e| e.to_string())?;
    if !smoke.passed {
        return Err(format!("refusing to install: smoke test failed — {}", smoke.detail));
    }

    let edn_path = brainbuilder_core::synthesis::install_synthesized(&component, &state.components_dir)
        .map_err(|e| e.to_string())?;

    // Hot-register so the palette updates without a restart.
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.install_component(&edn_path.to_string_lossy()).map_err(|e| e.to_string())
}

/// Auto-tuning: given a graph, sweep a small grid of training configs, run each
/// as a real (short) training trial, and return the trials ranked by final
/// loss — so a zero-knowledge user gets a config that already works instead of
/// guessing. Each trial's metrics still stream to the dashboard as it runs.
#[command]
async fn autotune(graph_json: String, budget: usize, search_arch: bool, state: State<'_, AppState>) -> Result<String, String> {
    use brainbuilder_core::autotune;

    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let base = graph
        .training
        .clone()
        .ok_or_else(|| "graph has no training config to tune".to_string())?;

    let space = {
        let s = autotune::SearchSpace::default_around(&base);
        if search_arch { s.with_architecture_search() } else { s }
    };
    let candidates = autotune::candidate_configs(&base, &space, budget.max(1));

    let mut results = Vec::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        let cfg = candidate.config;
        let width_scale = candidate.width_scale;
        let mut trial_graph = graph.clone();
        // Apply the architecture width scale to this trial's graph (no-op at 1.0).
        autotune::apply_width_scale(&mut trial_graph, width_scale);
        let batch_size = cfg.data_source.batch_size;
        let optimizer = cfg.optimizer.clone();
        let learning_rate = autotune::lr_of(&cfg);
        trial_graph.training = Some(cfg);

        // Capture the trial's best (lowest) loss by consuming the metrics
        // broadcast *concurrently* while training runs. Draining only after
        // completion risked losing points: the channel is bounded (200), so a
        // trial that emits more than that between our reads would overflow and
        // the late `try_recv` would see `Lagged` and stop early, scoring the
        // trial `None` (spuriously "failed"). Consuming as points arrive keeps
        // the receiver from ever lagging. Trials run sequentially, so points
        // don't interleave across trials.
        let mut rx = subscribe_metrics();
        let (stop_tx, mut stop_rx) = tokio::sync::oneshot::channel::<()>();
        let collector = tokio::spawn(async move {
            use tokio::sync::broadcast::error::RecvError;
            let mut best: Option<f32> = None;
            let mut note = |loss: f32| {
                best = Some(best.map_or(loss, |b| if loss < b { loss } else { b }));
            };
            loop {
                tokio::select! {
                    biased;
                    // Prefer draining metrics before honoring the stop signal so
                    // the final points of a trial are never missed.
                    r = rx.recv() => match r {
                        Ok(p) => note(p.loss),
                        Err(RecvError::Lagged(_)) => continue,
                        Err(RecvError::Closed) => break,
                    },
                    _ = &mut stop_rx => {
                        while let Ok(p) = rx.try_recv() {
                            note(p.loss);
                        }
                        break;
                    }
                }
            }
            best
        });

        let orchestrator = state.orchestrator.lock().await;
        let run = orchestrator.execute_graph(trial_graph).await;
        drop(orchestrator);

        // Signal the collector to finish and fold in any buffered points.
        let _ = stop_tx.send(());
        let collected = collector.await.unwrap_or(None);
        let score = match run {
            Ok(()) => collected,
            Err(_) => None,
        };

        results.push(autotune::TrialResult { index, learning_rate, batch_size, optimizer, width_scale, score });
    }

    let ranked = autotune::rank(&results);
    serde_json::to_string(&ranked).map_err(|e| e.to_string())
}

/// Resolve the git repo root from the components dir, so the agent's worktree
/// is cut from the real checkout the app is running out of.
fn repo_root_from(components_dir: &std::path::Path) -> Result<std::path::PathBuf, String> {
    let out = std::process::Command::new("git")
        .current_dir(components_dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("git not available: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository — the self-building agent needs one".to_string());
    }
    Ok(std::path::PathBuf::from(String::from_utf8_lossy(&out.stdout).trim().to_string()))
}

/// Start a self-building agent session: cut an isolated git worktree + branch
/// off the live checkout. `mode` is "propose-approve" | "auto-apply" | "full".
#[command]
async fn agent_start(mode: String, state: State<'_, AppState>) -> Result<String, String> {
    let mode: brainbuilder_core::agent::AutonomyMode =
        serde_json::from_str(&format!("\"{mode}\"")).map_err(|_| format!("unknown autonomy mode `{mode}`"))?;
    let repo_root = repo_root_from(&state.components_dir)?;
    let session = brainbuilder_core::agent::AgentSession::create(&repo_root, mode).map_err(|e| e.to_string())?;
    let json = serde_json::to_string(&session).map_err(|e| e.to_string())?;
    *state.agent.lock().await = Some(session);
    Ok(json)
}

/// The current agent session (isolated worktree/branch/mode), if any.
#[command]
async fn agent_status(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.agent.lock().await;
    match &*guard {
        Some(s) => Ok(Some(serde_json::to_string(s).map_err(|e| e.to_string())?)),
        None => Ok(None),
    }
}

/// Run one agent step on the active session: hand `task` to OpenCode inside the
/// sandboxed worktree, then run the test gate, then apply the mode's merge
/// policy. Returns a JSON report {agent_output, diff, gate_passed, gate_output,
/// merged}.
#[command]
async fn agent_run(task: String, selector: String, window: tauri::Window, state: State<'_, AppState>) -> Result<String, String> {
    let session = {
        let guard = state.agent.lock().await;
        guard.clone().ok_or_else(|| "no active agent session — start one first".to_string())?
    };

    // Blocking git/subprocess work off the async runtime. Each phase emits an
    // `agent-progress` event so the panel shows a live status + the diff as
    // soon as it's computed (before the slower test gate finishes), instead of
    // one opaque wait. The final `Ok` still returns the whole report as the
    // authoritative source of truth.
    let report = tokio::task::spawn_blocking(move || {
        let emit = |payload: serde_json::Value| {
            let _ = window.emit("agent-progress", payload);
        };

        emit(serde_json::json!({ "phase": "agent", "message": "OpenCode is editing in the isolated worktree…" }));
        let agent_output = session.run_agent_step(&task, &selector).unwrap_or_else(|e| format!("[agent step error] {e}"));
        emit(serde_json::json!({ "phase": "agent-output", "message": agent_output }));

        let diff = session.diff().unwrap_or_default();
        emit(serde_json::json!({ "phase": "diff", "diff": diff }));

        emit(serde_json::json!({ "phase": "tests", "message": "Running the test gate in the worktree…" }));
        let gate = session.run_test_gate();
        emit(serde_json::json!({ "phase": "gate", "gate_passed": gate.passed, "gate_output": gate.output }));

        // Merge policy: propose never auto-merges; auto merges on green; full
        // merges regardless (test failures still recorded in the gate output).
        let mut merged = false;
        if session.mode.merges_automatically() && (gate.passed || !session.mode.requires_green_tests()) {
            if session.approve().is_ok() {
                merged = true;
            }
        }
        emit(serde_json::json!({ "phase": "done", "merged": merged }));

        serde_json::json!({
            "agent_output": agent_output,
            "diff": diff,
            "gate_passed": gate.passed,
            "gate_output": gate.output,
            "merged": merged,
        })
        .to_string()
    })
    .await
    .map_err(|e| e.to_string())?;

    Ok(report)
}

/// Approve (merge) the active session's work into the live checkout — the
/// human gate for propose-approve mode.
#[command]
async fn agent_approve(state: State<'_, AppState>) -> Result<(), String> {
    let session = {
        let guard = state.agent.lock().await;
        guard.clone().ok_or_else(|| "no active agent session".to_string())?
    };
    tokio::task::spawn_blocking(move || session.approve())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// Discard the active session's worktree + branch (a true undo — nothing was
/// merged) and clear it.
#[command]
async fn agent_revert(state: State<'_, AppState>) -> Result<(), String> {
    let session = {
        let mut guard = state.agent.lock().await;
        guard.take().ok_or_else(|| "no active agent session".to_string())?
    };
    tokio::task::spawn_blocking(move || session.revert())
        .await
        .map_err(|e| e.to_string())?
        .map_err(|e| e.to_string())
}

/// The task-first Intent layer's front door: given a goal (classify/regress)
/// and a pointer at real data, inspect the data and return a validated,
/// trainable model proposal — the on-ramp for someone who thinks in outcomes,
/// not graphs. Returns the whole `ProposedModel` (graph + class names + sizes
/// + a plain-English rationale) as JSON.
///
/// Same non-`Send` registry-lock dance as `generate_graph`: inspect the data
/// asynchronously (through DataFusion) with no lock held, then acquire the
/// registry guard only for the synchronous, no-await finalize + validate step.
#[command]
async fn propose_model(request_json: String, state: State<'_, AppState>) -> Result<String, String> {
    let request: brainbuilder_core::intent::IntentRequest =
        serde_json::from_str(&request_json).map_err(|e| e.to_string())?;

    let shape = brainbuilder_core::intent::inspect_data(&request).await.map_err(|e| e.to_string())?;

    let orchestrator = state.orchestrator.lock().await;
    let registry = orchestrator
        .context
        .registry
        .read()
        .map_err(|_| "component registry lock poisoned".to_string())?;
    let proposal =
        brainbuilder_core::intent::finalize_proposal(&request, shape, &registry).map_err(|e| e.to_string())?;
    serde_json::to_string(&proposal).map_err(|e| e.to_string())
}

/// Transfer-learning front door: adapt a real pretrained `.safetensors`
/// backbone to the user's data (frozen backbone + fresh trainable head).
/// Returns a validated `ProposedModel` as JSON. Same non-`Send` registry-lock
/// split as `propose_model`: inspect the file + data asynchronously, then
/// finalize + validate under the lock with no await.
#[command]
async fn propose_transfer_model(request_json: String, state: State<'_, AppState>) -> Result<String, String> {
    let request: brainbuilder_core::intent::TransferRequest =
        serde_json::from_str(&request_json).map_err(|e| e.to_string())?;

    let shape = brainbuilder_core::intent::inspect_transfer(&request).await.map_err(|e| e.to_string())?;

    let orchestrator = state.orchestrator.lock().await;
    let registry = orchestrator
        .context
        .registry
        .read()
        .map_err(|_| "component registry lock poisoned".to_string())?;
    let proposal = brainbuilder_core::intent::finalize_transfer(&request, shape, &registry).map_err(|e| e.to_string())?;
    serde_json::to_string(&proposal).map_err(|e| e.to_string())
}

/// Data-time diagnostics: read the real data the user pointed at and return a
/// plain-English list of statistical problems (class imbalance, tiny classes,
/// numeric feature/target leakage) before they commit to a training run — the
/// "will this even work?" check that shape validation can't give. No registry
/// needed, so it's purely async I/O.
#[command]
async fn diagnose_data(request_json: String) -> Result<Vec<brainbuilder_core::diagnostics::Diagnostic>, String> {
    let request: brainbuilder_core::intent::IntentRequest =
        serde_json::from_str(&request_json).map_err(|e| e.to_string())?;
    brainbuilder_core::intent::diagnose_data(&request).await.map_err(|e| e.to_string())
}

/// Training-time diagnostics: interpret a loss curve in plain English (diverged
/// / not learning / learning well) with a concrete suggested fix. Pure — the
/// GUI passes the losses it already streamed from the Metrics tab.
#[command]
async fn diagnose_training(losses: Vec<f32>) -> Result<Vec<brainbuilder_core::diagnostics::Diagnostic>, String> {
    Ok(brainbuilder_core::diagnostics::analyze_loss_curve(&losses))
}

/// Real local models already on this machine: the HuggingFace hub cache
/// (always scanned), plus any user-configured extra directories (e.g. a
/// personal `D:\Models\general` folder that isn't hub-cache-shaped — see
/// `models::discovery::scan_directory_for_models`). `extra_dirs` comes from
/// the GUI's persisted (localStorage) directory list, not a hardcoded path.
#[command]
async fn list_local_models(extra_dirs: Vec<String>) -> Result<Vec<brainbuilder_core::models::LocalModel>, String> {
    let cache = brainbuilder_core::models::default_hf_cache_dir();
    let mut models = brainbuilder_core::models::scan_local_models(&cache);
    for dir in extra_dirs {
        models.extend(brainbuilder_core::models::scan_directory_for_models(std::path::Path::new(&dir), 4));
    }
    Ok(models)
}

/// Real tensor manifest (name/shape/dtype) for a `.safetensors` file, no
/// weight data loaded — instant even for a multi-gigabyte model.
#[command]
async fn inspect_safetensors(path: String) -> Result<Vec<(String, Vec<i64>, String)>, String> {
    brainbuilder_core::models::safetensors_loader::list_safetensors_manifest(std::path::Path::new(&path))
        .map_err(|e| e.to_string())
}

/// Real ONNX Runtime session metadata (input/output tensor specs) for an
/// `.onnx` file.
#[command]
async fn inspect_onnx(path: String) -> Result<(Vec<brainbuilder_core::models::onnx_loader::OnnxIoSpec>, Vec<brainbuilder_core::models::onnx_loader::OnnxIoSpec>), String> {
    brainbuilder_core::models::onnx_loader::inspect_onnx_model(std::path::Path::new(&path)).map_err(|e| e.to_string())
}

/// Registers a local `.gguf` file with Ollama under `model_name` (real
/// `ollama create` subprocess call — see `models::gguf_router`), so it can
/// be chatted with over Ollama's own local API afterward.
#[command]
async fn register_gguf_model(path: String, model_name: String) -> Result<(), String> {
    let router = brainbuilder_core::models::GgufRouter::default();
    router.register_local_gguf(std::path::Path::new(&path), &model_name).map_err(|e| e.to_string())
}

/// The nervous system's observability tap: every sandboxed subprocess
/// invocation (Racket/Clojure via `Supervisor`, Python via its persistent
/// worker) is recorded — allowed, capability-denied, or timeout-killed —
/// so the Console can show a person what sandboxed component code actually
/// did, instead of that being an invisible internal detail.
#[command]
async fn get_nervous_system_audit(limit: i64) -> Result<Vec<brainbuilder_core::runtime::nervous_system::AuditRecord>, String> {
    Ok(brainbuilder_core::runtime::nervous_system::audit::recent(limit))
}

#[command]
async fn execute_graph(graph_json: String, state: State<'_, AppState>) -> Result<(), String> {
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    log::info!("executing graph `{}` ({})", graph.name, graph.graph_id);

    // Tap the same broadcast metrics stream the GUI's live chart listens to
    // with a second, independent subscriber — broadcast channels queue every
    // message for each subscriber separately, so this never drops or steals
    // events the chart needs. Captures just the first/last loss so a finished
    // run can be logged to the experiment history below.
    let mut metrics_rx = brainbuilder_core::data::metrics::subscribe_metrics();
    let captured: Arc<std::sync::Mutex<(Option<f32>, Option<f32>)>> = Arc::new(std::sync::Mutex::new((None, None)));
    let captured_task = captured.clone();
    let capture_handle = tauri::async_runtime::spawn(async move {
        while let Ok(point) = metrics_rx.recv().await {
            if let Ok(mut c) = captured_task.lock() {
                if c.0.is_none() {
                    c.0 = Some(point.loss);
                }
                c.1 = Some(point.loss);
            }
        }
    });

    let orchestrator = state.orchestrator.lock().await;
    let result = orchestrator.execute_graph(graph.clone()).await;
    capture_handle.abort();

    if let Err(e) = &result {
        log::error!("graph execution failed: {e}");
        return Err(e.to_string());
    }

    if let Some(training) = &graph.training {
        let (first_loss, last_loss) = captured.lock().map(|c| *c).unwrap_or((None, None));
        let architecture = graph.nodes.iter().map(|n| n.component.as_str()).collect::<Vec<_>>().join(" → ");
        let record = brainbuilder_core::utils::experiment_log::NewExperiment {
            graph_id: graph.graph_id.clone(),
            graph_name: graph.name.clone(),
            architecture,
            loss_fn: training.loss.clone(),
            optimizer: training.optimizer.clone(),
            lr: training.hyperparams.get("lr").and_then(|v| v.as_f64()).unwrap_or(0.0),
            batch_size: training.data_source.batch_size as i64,
            epochs: training.hyperparams.get("epochs").and_then(|v| v.as_i64()).unwrap_or(0),
            first_loss: first_loss.map(|f| f as f64),
            last_loss: last_loss.map(|f| f as f64),
        };
        if let Err(e) = orchestrator.context.experiments.log(record) {
            // Non-fatal: the run itself succeeded, only its history entry
            // failed to write — don't fail a successful training run over it.
            log::warn!("failed to log experiment record: {e}");
        }
    }

    Ok(())
}

#[command]
async fn list_experiments(
    limit: usize,
    state: State<'_, AppState>,
) -> Result<Vec<brainbuilder_core::utils::experiment_log::ExperimentRecord>, String> {
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.context.experiments.recent(limit).map_err(|e| e.to_string())
}

#[command]
async fn get_components(state: State<'_, AppState>) -> Result<Vec<String>, String> {
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.list_components().map_err(|e| e.to_string())
}

#[command]
async fn get_component_descriptors(
    state: State<'_, AppState>,
) -> Result<Vec<brainbuilder_core::component::descriptor::ComponentSummary>, String> {
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.component_summaries().map_err(|e| e.to_string())
}

#[command]
async fn save_graph(path: String, graph_json: String) -> Result<(), String> {
    let graph: brainbuilder_core::bbir::BBIRGraph =
        serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let edn = graph.to_edn().map_err(|e| e.to_string())?;
    std::fs::write(&path, edn).map_err(|e| e.to_string())
}

#[command]
async fn load_graph(path: String) -> Result<String, String> {
    let edn = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
    let graph = brainbuilder_core::bbir::BBIRGraph::from_edn(&edn).map_err(|e| e.to_string())?;
    serde_json::to_string(&graph).map_err(|e| e.to_string())
}

#[command]
async fn preview_dataset(
    path: String,
    limit: usize,
) -> Result<brainbuilder_core::data::source::DatasetPreview, String> {
    brainbuilder_core::data::source::preview_dataset(&path, limit)
        .await
        .map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
struct PredictResult {
    shape: Vec<i64>,
    values: Vec<f32>,
}

#[command]
async fn predict(
    graph_json: String,
    dataset_path: String,
    rows: usize,
    state: State<'_, AppState>,
) -> Result<Vec<PredictResult>, String> {
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let batch = brainbuilder_core::data::source::load_batch(&dataset_path, rows)
        .await
        .map_err(|e| e.to_string())?;
    let orchestrator = state.orchestrator.lock().await;
    let tensors = orchestrator.predict(graph, batch).map_err(|e| e.to_string())?;
    tensors
        .iter()
        .map(|t| {
            brainbuilder_core::interop::dlpack_support::tensor_to_vec_f32(t)
                .map(|(shape, values)| PredictResult { shape, values })
                .map_err(|e| e.to_string())
        })
        .collect()
}

/// Ranks each feature column of a tabular dataset by how much predictions
/// move when that column is decoupled from its rows — see
/// `interpret::feature_importance` for the real method behind this.
#[command]
async fn feature_importance(
    graph_json: String,
    dataset_path: String,
    rows: usize,
    state: State<'_, AppState>,
) -> Result<Vec<brainbuilder_core::interpret::FeatureImportance>, String> {
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let batch = brainbuilder_core::data::source::load_batch(&dataset_path, rows)
        .await
        .map_err(|e| e.to_string())?;
    let orchestrator = state.orchestrator.lock().await;
    brainbuilder_core::interpret::feature_importance(&orchestrator, &graph, batch).map_err(|e| e.to_string())
}

/// Runs the trained checkpoint over every row of `dataset_path` (unlike
/// `predict`, no row cap) and writes `<input columns...>,prediction` to
/// `output_path` as CSV — see `batch_predict::run_batch_predict`. Returns the
/// number of rows written so the GUI can report a real count back.
#[command]
async fn batch_predict(
    graph_json: String,
    dataset_path: String,
    output_path: String,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let graph: BBIRGraph = serde_json::from_str(&graph_json).map_err(|e| e.to_string())?;
    let orchestrator = state.orchestrator.lock().await;
    brainbuilder_core::batch_predict::run_batch_predict(
        &orchestrator,
        &graph,
        &dataset_path,
        std::path::Path::new(&output_path),
    )
    .await
    .map_err(|e| e.to_string())
}

/// Starts a real local HTTP server (loopback-only, see `predict_server.rs`)
/// answering `POST /predict` against the trained checkpoint — the
/// live-serving counterpart to "Export checkpoint…"'s file hand-off. A
/// second call while one is already running just returns the existing URL
/// rather than binding a second port.
#[command]
async fn start_predict_server(app: tauri::AppHandle, state: State<'_, AppState>) -> Result<String, String> {
    let mut guard = state.predict_server.lock().await;
    if let Some((url, _)) = guard.as_ref() {
        return Ok(url.clone());
    }
    let (url, shutdown) = predict_server::spawn(app);
    *guard = Some((url.clone(), shutdown));
    Ok(url)
}

/// Gracefully shuts down the running predict server, if any. A no-op (not an
/// error) if none is running.
#[command]
async fn stop_predict_server(state: State<'_, AppState>) -> Result<(), String> {
    let mut guard = state.predict_server.lock().await;
    if let Some((_, shutdown)) = guard.take() {
        let _ = shutdown.send(());
    }
    Ok(())
}

/// The running predict server's URL, if one is currently up — lets the GUI
/// recover its state after a reload without starting a second server.
#[command]
async fn predict_server_status(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let guard = state.predict_server.lock().await;
    Ok(guard.as_ref().map(|(url, _)| url.clone()))
}

#[command]
async fn has_checkpoint(graph_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let orchestrator = state.orchestrator.lock().await;
    Ok(orchestrator.has_checkpoint(&graph_id))
}

/// Copies a trained checkpoint out of BrainBuilder's internal `checkpoints/`
/// folder to wherever the user chooses — the hand-off point for using a
/// model outside the app. The file is a genuine, standard PyTorch state-dict
/// (`torch.save({name: tensor, ...}, path)`, see
/// `components/python/_bb_worker.py`'s `handle_save_state_dict`), loadable
/// anywhere with plain `torch.load()`; nothing BrainBuilder-specific about
/// the format itself.
#[command]
async fn export_checkpoint(graph_id: String, dest_path: String, state: State<'_, AppState>) -> Result<(), String> {
    let orchestrator = state.orchestrator.lock().await;
    if !orchestrator.has_checkpoint(&graph_id) {
        return Err("No trained checkpoint exists yet for this graph — train it first.".to_string());
    }
    let src = orchestrator.context.checkpoint_path(&graph_id);
    std::fs::copy(&src, &dest_path)
        .map(|_| ())
        .map_err(|e| format!("Couldn't copy the checkpoint to `{dest_path}`: {e}"))
}

/// Every archived checkpoint version for a graph, newest first — see
/// `checkpoint_versions::archive_current`, called automatically right before
/// each training run would otherwise overwrite the current checkpoint.
#[command]
async fn list_checkpoint_versions(
    graph_id: String,
    state: State<'_, AppState>,
) -> Result<Vec<brainbuilder_core::utils::checkpoint_versions::CheckpointVersion>, String> {
    let orchestrator = state.orchestrator.lock().await;
    brainbuilder_core::utils::checkpoint_versions::list_versions(&orchestrator.context.checkpoints_dir, &graph_id)
        .map_err(|e| e.to_string())
}

/// Restores an archived version as the current checkpoint. The checkpoint it
/// replaces is archived first, so this is itself undoable — never a one-way
/// door.
#[command]
async fn restore_checkpoint_version(
    graph_id: String,
    version_id: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let orchestrator = state.orchestrator.lock().await;
    brainbuilder_core::utils::checkpoint_versions::restore_version(
        &orchestrator.context.checkpoints_dir,
        &graph_id,
        &version_id,
    )
    .map_err(|e| e.to_string())
}

#[command]
async fn install_component(path: String, state: State<'_, AppState>) -> Result<(), String> {
    log::info!("installing component from {path}");
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.install_component(&path).map_err(|e| {
        log::error!("component install failed: {e}");
        e.to_string()
    })
}

/// Stream training metrics to the frontend via Tauri events.
fn setup_metrics_event(app: &mut tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle().clone();
    let mut rx = subscribe_metrics();
    // `.setup()` runs on the main thread outside any Tokio reactor context,
    // so `tokio::spawn` panics here ("no reactor running"). Tauri drives its
    // own managed async runtime (tokio, by default) for command dispatch —
    // `tauri::async_runtime::spawn` schedules onto that instead.
    tauri::async_runtime::spawn(async move {
        while let Ok(point) = rx.recv().await {
            let _ = handle.emit_all("metrics-update", point);
        }
    });
    Ok(())
}

fn path_sep() -> &'static str {
    if cfg!(windows) { ";" } else { ":" }
}

/// Starts the Cluster actor (real libp2p swarm, see `cluster_actor.rs`) and
/// fills `cell` once it's ready. Identity is persisted under the app's data
/// dir so this device's `PeerId` survives restarts, per Phase 3
/// (`CLUSTER_PLAN.md`). Once the actor is up, also starts the observer HTTP
/// server (Phase 5's phone/tablet view, `observer_server.rs`) against the
/// same live handle.
fn setup_cluster(
    app: &mut tauri::App,
    cluster_cell: Arc<OnceCell<ClusterHandle>>,
    observer_cell: Arc<OnceCell<String>>,
    context: Arc<brainbuilder_core::AppContext>,
) -> Result<(), Box<dyn std::error::Error>> {
    let identity_path = app
        .path_resolver()
        .app_data_dir()
        .unwrap_or_else(std::env::temp_dir)
        .join("cluster_identity.key");
    tauri::async_runtime::spawn(async move {
        let handle = cluster_actor::spawn(identity_path, context).await;
        let url = observer_server::spawn(handle.clone()).await;
        observer_cell.set(url).ok();
        cluster_cell.set(handle).ok();
    });
    Ok(())
}

fn main() {
    env_logger::init();

    // WebView2 defaults to one shared profile directory
    // (`%LOCALAPPDATA%\EBWebView`) for every Tauri/wry app on the machine
    // that doesn't override it. Since all Tauri apps also share the same
    // fixed `http://tauri.localhost/` origin, that means a service worker
    // (or any other origin-scoped storage) registered by a *different*
    // local Tauri app can silently intercept BrainBuilder's own page loads —
    // observed directly: a stale service worker from an unrelated app was
    // serving its own UI in place of BrainBuilder's, with every Tauri IPC
    // command failing "not found" as a result, since the two apps'
    // `invoke_handler`s are naturally different. Giving this app its own
    // profile directory closes that off entirely. Must be set before the
    // webview is created (i.e. before `tauri::Builder::run`), and only
    // matters on Windows — other platforms' webviews don't read it.
    #[cfg(target_os = "windows")]
    if let Ok(local_app_data) = std::env::var("LOCALAPPDATA") {
        std::env::set_var(
            "WEBVIEW2_USER_DATA_FOLDER",
            std::path::Path::new(&local_app_data).join("BrainBuilder").join("EBWebView"),
        );
    }

    // Components directory is resolved relative to the executable's working
    // directory at dev time; for a packaged build this should instead be
    // bundled as a Tauri resource and resolved via `tauri::api::path`. The
    // cwd differs depending on how dev is launched — `tauri dev`'s
    // `beforeDevCommand`/cargo invocation runs from `gui/src-tauri`
    // (needing `../../components`), while running the built exe directly
    // from `gui/` needs `../components` — so try both instead of assuming.
    let cwd = std::env::current_dir().expect("cannot read cwd");
    let components_dir = [cwd.join("../components"), cwd.join("../../components")]
        .into_iter()
        .find(|p| p.is_dir())
        .unwrap_or_else(|| cwd.join("../components"));

    // Real component implementations (components/python/*.py, e.g. linear.py,
    // relu.py) must be importable by module name (PythonBridge imports
    // `linear`, `relu`, ... — the component's registry name, not any
    // descriptor `:path`). pyo3's embedded interpreter reads PYTHONPATH from
    // the process environment at its first use, so this must be set before
    // any `Python::with_gil` call happens.
    let python_components_dir = components_dir.join("python");
    match std::env::var("PYTHONPATH") {
        Ok(existing) => std::env::set_var(
            "PYTHONPATH",
            format!("{}{}{}", python_components_dir.display(), path_sep(), existing),
        ),
        Err(_) => std::env::set_var("PYTHONPATH", &python_components_dir),
    }

    let orchestrator = Orchestrator::new(&components_dir)
        .expect("Failed to initialise orchestrator. Ensure components/ directory exists.");
    // Cloned before `orchestrator` moves into `AppState` below — the
    // Cluster actor compiles distributed jobs against this same registry
    // and tensor arena, so a hosted/joined graph resolves identically to
    // the app's own local training path.
    let cluster_context = orchestrator.context.clone();

    let cluster_cell = Arc::new(OnceCell::new());
    let observer_cell = Arc::new(OnceCell::new());
    let bot_handle = Arc::new(Mutex::new(bot_dashboard::BotServerHandle::default()));

    tauri::Builder::default()
        .manage(AppState {
            orchestrator: Mutex::new(orchestrator),
            cluster: cluster_cell.clone(),
            observer_url: observer_cell.clone(),
            components_dir: components_dir.clone(),
            agent: Mutex::new(None),
            predict_server: Mutex::new(None),
            bot: bot_handle.clone(),
            debug: Arc::new(Mutex::new(webview_debug::DebugState::default())),
        })
        .manage(crate::nervous_system::commands::NervousSystemState {
            registry: crate::nervous_system::registry::ProviderRegistry::new(),
            system: crate::nervous_system::nervous_system::NervousSystem::new(),
        })
        .setup(move |app| {
            setup_metrics_event(app)?;
            setup_cluster(app, cluster_cell.clone(), observer_cell.clone(), cluster_context.clone())?;
            app.manage(crate::tool_executor::commands::ToolExecutorState {
                registry: crate::tool_executor::registry::ToolRegistry::new(),
            });
            let _ = bot_dashboard::try_autostart(&bot_handle);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            validate_graph,
            execute_graph,
            list_experiments,
            get_components,
            get_component_descriptors,
            preview_dataset,
            save_graph,
            load_graph,
            predict,
            batch_predict,
            start_predict_server,
            stop_predict_server,
            predict_server_status,
            feature_importance,
            has_checkpoint,
            export_checkpoint,
            list_checkpoint_versions,
            restore_checkpoint_version,
            install_component,
            get_nervous_system_audit,
            get_cluster_status,
            create_cluster,
            generate_pairing_code,
            join_cluster_with_code,
            host_distributed_job,
            list_distributed_jobs,
            join_distributed_job,
            get_distributed_training_status,
            get_observer_url,
            generate_graph,
            synthesize_component,
            install_synthesized_component,
            agent_start,
            agent_status,
            agent_run,
            agent_approve,
            agent_revert,
            autotune,
            list_llm_providers,
            list_provider_models,
            list_gpu_adapters,
            probe_gpu_adapter,
            set_preferred_gpu,
            set_provider_credentials,
            has_provider_credentials,
            propose_model,
            propose_transfer_model,
            diagnose_data,
            diagnose_training,
            list_local_models,
            inspect_safetensors,
            inspect_onnx,
            register_gguf_model,
            pm_health,
            pm_list_domains,
            pm_apply_power_limit,
            pm_recent_telemetry,
            pm_list_blueprints,
            pm_build_mpc_spec,
            ca_health,
            ca_compress,
            ca_recent_stats,
            ca_list_blueprints,
            ca_build_compressor_spec,
            luci_status,
            luci_greet,
            luci_chat,
            luci_propose_plan,
            luci_list_plans,
            luci_update_plan_status,
            luci_reflect,
            luci_recent_reflections,
            luci_set_preference,
            luci_get_preference,
            luci_remember_fact,
            luci_recall_memories,
            luci_forget_memory,
            luci_audit,
            luci_recent_audit,
            luci_register_tool,
            luci_improve,
            self_improving_commands::luci_register_skill,
            self_improving_commands::luci_list_skills,
            self_improving_commands::luci_observe_and_learn,
            self_improving_commands::luci_imitate_skill,
            self_improving_commands::luci_decompose_task,
            self_improving_commands::luci_register_model,
            self_improving_commands::luci_list_models,
            self_improving_commands::luci_register_dataset,
            self_improving_commands::luci_list_datasets,
            self_improving_commands::luci_start_training,
            self_improving_commands::luci_list_training_jobs,
            self_improving_commands::bot_start_adapter,
            self_improving_commands::bot_list_adapters,
            self_improving_commands::bot_adapter_health,
            self_improving_commands::bot_send_message,
            self_improving_commands::bot_start_call,
            self_improving_commands::bot_end_call,
            tool_executor_run,
            tool_executor_list,
            bot_dashboard_status,
            bot_dashboard_settings_get,
            bot_dashboard_settings_set,
            bot_dashboard_start,
            bot_dashboard_stop,
            bot_dashboard_restart,
            bot_dashboard_events,
            bot_dashboard_clear_events,
            bot_dashboard_telemetry,
            bot_autostart,
            webview_debug_eval,
            webview_debug_query,
            webview_debug_click,
            webview_debug_fill,
            webview_debug_snapshot,
            webview_debug_get_state,
            webview_debug_set_enabled,
            webview_debug_is_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

