#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod cluster_actor;
mod observer_server;

use tauri::{command, Manager, State};
use brainbuilder_core::orchestrator::Orchestrator;
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::data::metrics::subscribe_metrics;
use cluster_actor::{ClusterHandle, ClusterStatus};
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
#[command]
async fn generate_graph(description: String, model: String, state: State<'_, AppState>) -> Result<String, String> {
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

    let client = brainbuilder_core::llm::OllamaClient::new();
    let raw = client.generate_json(&model, &system, &description).await.map_err(|e| e.to_string())?;

    let orchestrator = state.orchestrator.lock().await;
    let registry = orchestrator.context.registry.read().map_err(|_| "component registry lock poisoned".to_string())?;
    let graph = brainbuilder_core::llm::parse_and_validate(&raw, &registry).map_err(|e| e.to_string())?;
    serde_json::to_string(&graph).map_err(|e| e.to_string())
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
    let orchestrator = state.orchestrator.lock().await;
    orchestrator.execute_graph(graph).await.map_err(|e| {
        log::error!("graph execution failed: {e}");
        e.to_string()
    })
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

#[command]
async fn has_checkpoint(graph_id: String, state: State<'_, AppState>) -> Result<bool, String> {
    let orchestrator = state.orchestrator.lock().await;
    Ok(orchestrator.has_checkpoint(&graph_id))
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

    tauri::Builder::default()
        .manage(AppState {
            orchestrator: Mutex::new(orchestrator),
            cluster: cluster_cell.clone(),
            observer_url: observer_cell.clone(),
        })
        .setup(move |app| {
            setup_metrics_event(app)?;
            setup_cluster(app, cluster_cell.clone(), observer_cell.clone(), cluster_context.clone())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            validate_graph,
            execute_graph,
            get_components,
            get_component_descriptors,
            preview_dataset,
            save_graph,
            load_graph,
            predict,
            has_checkpoint,
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
            list_local_models,
            inspect_safetensors,
            inspect_onnx,
            register_gguf_model
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
