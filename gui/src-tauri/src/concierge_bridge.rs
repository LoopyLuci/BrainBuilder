//! Tauri command handlers bridging frontend ↔ ConciergeAgent + PlatformHost + ModelExecutor.

use crate::model_executor::ModelExecutor;
use crate::platform_host::OmniForgeHost;
use concierge_core::ConciergeAgent;

use std::sync::Arc;
use tauri::State;
use tokio::sync::Mutex;

pub struct AppState {
    pub concierge: Mutex<ConciergeAgent>,
    pub host: Arc<OmniForgeHost>,
    pub executor: Arc<ModelExecutor>,
}

#[tauri::command]
pub async fn concierge_chat(
    state: State<'_, AppState>,
    message: String,
) -> Result<String, String> {
    let mut agent = state.concierge.lock().await;
    agent.chat(&message).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_canvas(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let snap = state.host.get_canvas_snapshot().await;
    let nodes: Vec<_> = snap
        .nodes
        .iter()
        .map(|n| {
            serde_json::json!({
                "id": n.id,
                "type": "default",
                "position": { "x": n.x, "y": n.y },
                "data": {
                    "label": n.label,
                    "nodeType": n.node_type,
                    "modelId": n.model_id
                }
            })
        })
        .collect();
    let edges: Vec<_> = snap
        .edges
        .iter()
        .map(|e| {
            serde_json::json!({
                "id": e.id,
                "source": e.source,
                "target": e.target,
                "sourceHandle": e.source_socket,
                "targetHandle": e.target_socket
            })
        })
        .collect();
    Ok(serde_json::json!({ "nodes": nodes, "edges": edges }))
}

#[tauri::command]
pub async fn import_model(
    state: State<'_, AppState>,
    path: String,
    format: Option<String>,
    name: Option<String>,
) -> Result<String, String> {
    state
        .host
        .import_model(&path, format.as_deref(), name.as_deref())
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_node(
    state: State<'_, AppState>,
    node_type: String,
    model_id: Option<String>,
    label: Option<String>,
    x: f64,
    y: f64,
    config: Option<serde_json::Value>,
) -> Result<String, String> {
    state
        .host
        .add_node(
            &node_type,
            model_id.as_deref(),
            label.as_deref(),
            x,
            y,
            config,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn connect_nodes(
    state: State<'_, AppState>,
    source_node: String,
    source_socket: String,
    target_node: String,
    target_socket: String,
) -> Result<String, String> {
    state
        .host
        .connect_nodes(&source_node, &source_socket, &target_node, &target_socket)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn inspect_node(
    state: State<'_, AppState>,
    node_id: String,
) -> Result<String, String> {
    state
        .host
        .inspect_node(&node_id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_graph(
    state: State<'_, AppState>,
    inputs: serde_json::Value,
) -> Result<String, String> {
    state
        .host
        .execute_graph(&inputs, None)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn run_training(
    state: State<'_, AppState>,
    base_model: String,
    dataset: String,
    recipe: String,
    output_name: Option<String>,
    epochs: Option<i64>,
    learning_rate: Option<f64>,
) -> Result<String, String> {
    state
        .host
        .run_training(
            &base_model,
            &dataset,
            &recipe,
            output_name.as_deref(),
            epochs,
            learning_rate,
        )
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_platform_status(state: State<'_, AppState>) -> Result<String, String> {
    state
        .host
        .get_platform_status()
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn list_models(
    state: State<'_, AppState>,
    limit: Option<usize>,
) -> Result<String, String> {
    state
        .host
        .list_models(limit.unwrap_or(50))
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_models(
    state: State<'_, AppState>,
    query: String,
    modality: Option<String>,
    arch: Option<String>,
) -> Result<String, String> {
    state
        .host
        .search_models(&query, modality.as_deref(), arch.as_deref())
        .await
        .map_err(|e| e.to_string())
}

// ── Model execution commands ────────────────────────────────────────────

#[tauri::command]
pub async fn load_onnx_model(
    state: State<'_, AppState>,
    path: String,
) -> Result<String, String> {
    state.executor.load_onnx(&path).await
}

#[tauri::command]
pub async fn infer_onnx(
    state: State<'_, AppState>,
    session_id: String,
    inputs: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let result = state.executor.infer_onnx(&session_id, inputs).await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn start_gguf_model(
    state: State<'_, AppState>,
    path: String,
    port: Option<u16>,
) -> Result<String, String> {
    state.executor.start_gguf(&path, port.unwrap_or(8080)).await
}

#[tauri::command]
pub async fn stop_model(
    state: State<'_, AppState>,
    process_id: String,
) -> Result<(), String> {
    state.executor.stop(&process_id).await
}

#[tauri::command]
pub async fn list_running_models(
    state: State<'_, AppState>,
) -> Result<Vec<String>, String> {
    Ok(state.executor.list().await)
}

#[tauri::command]
pub async fn list_gguf_models(
    state: State<'_, AppState>,
) -> Result<Vec<serde_json::Value>, String> {
    let infos = state.executor.list_gguf().await;
    Ok(infos
        .into_iter()
        .map(|i| {
            serde_json::json!({
                "id": i.id,
                "model_path": i.model_path,
                "port": i.port,
                "endpoint": i.endpoint,
                "status": i.status,
            })
        })
        .collect())
}


// ── Extended production capabilities ────────────────────────────────────

use crate::dataset_tools;
use crate::export_bundle;
use crate::inference_sandbox;
use crate::km_format;
use crate::multimodal_merger::{self, MultimodalConfig, MultimodalInput};
use crate::plugin_system;
use crate::training_executor::{self, TrainingConfig};

#[tauri::command]
pub async fn start_training(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    config: TrainingConfig,
) -> Result<String, String> {
    let _ = &state; // keep state available for future job tracking
    training_executor::start_training_job_lenient(config, app).await
}

#[tauri::command]
pub async fn create_km(
    base_model: String,
    adapter_path: String,
    name: String,
    rank: Option<u32>,
    alpha: Option<f32>,
    output_path: Option<String>,
) -> Result<String, String> {
    let out = output_path.unwrap_or_else(|| format!("{name}.km"));
    km_format::package_adapter_dir(
        &adapter_path,
        &name,
        &base_model,
        rank.unwrap_or(16),
        alpha.unwrap_or(32.0),
        &out,
    )
}

#[tauri::command]
pub async fn export_model_bundle(
    base_model: String,
    km_paths: Vec<String>,
    graph: String,
    output: String,
) -> Result<(), String> {
    export_bundle::create_bundle(&base_model, &km_paths, &graph, &output)
}

#[tauri::command]
pub fn get_plugins() -> Vec<plugin_system::PluginManifest> {
    plugin_system::discover_plugins()
}

#[tauri::command]
pub async fn run_plugin_tool(
    plugin: String,
    tool: String,
    arguments: String,
) -> Result<String, String> {
    plugin_system::execute_plugin_tool(&plugin, &tool, &arguments).await
}

#[tauri::command]
pub async fn augment_dataset(
    path: String,
    methods: Vec<String>,
) -> Result<String, String> {
    dataset_tools::augment_dataset(&path, &methods).await
}

#[tauri::command]
pub async fn process_multimodal(
    state: State<'_, AppState>,
    config: MultimodalConfig,
    input: MultimodalInput,
) -> Result<serde_json::Value, String> {
    let out = multimodal_merger::process_multimodal(&config, &input, &state.executor).await?;
    serde_json::to_value(out).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn execute_canvas_graph(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    graph: serde_json::Value,
) -> Result<serde_json::Value, String> {
    inference_sandbox::execute_graph(&graph, &state.host, &state.executor, &app).await
}


#[tauri::command]
pub async fn rag_retrieve(
    source: String,
    query: String,
    k: Option<usize>,
) -> Result<serde_json::Value, String> {
    let result = crate::rag::retrieve(&source, &query, k.unwrap_or(4)).await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn rag_index_km(km_path: String) -> Result<String, String> {
    crate::rag::index_km(&km_path).await
}


#[tauri::command]
pub async fn hybrid_search(
    source: String,
    query: String,
    k: Option<usize>,
    alpha: Option<f32>,
) -> Result<serde_json::Value, String> {
    let result = crate::rag::retrieve_hybrid(
        &source,
        &query,
        k.unwrap_or(5),
        alpha.unwrap_or(0.6),
    )
    .await?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn vector_upsert(
    items: Vec<(String, String)>,
) -> Result<usize, String> {
    // Ephemeral in-memory upsert for experimentation from the UI
    use crate::vector_store::VectorStore;
    use std::collections::HashMap;
    let store = VectorStore::memory();
    let mapped = items
        .into_iter()
        .map(|(id, text)| (id, text, HashMap::new()))
        .collect();
    store.upsert_texts(mapped).await
}


#[tauri::command]
pub fn list_editable_sources() -> Result<Vec<crate::self_edit::FileEntry>, String> {
    crate::self_edit::list_editable_sources()
}

#[tauri::command]
pub fn read_source_file(path: String) -> Result<String, String> {
    crate::self_edit::read_source_file(&path)
}

#[tauri::command]
pub fn atomic_write_source(
    app: tauri::AppHandle,
    path: String,
    content: String,
    trigger_reload: Option<bool>,
) -> Result<(), String> {
    crate::self_edit::atomic_write_source(
        &path,
        &content,
        Some(&app),
        trigger_reload.unwrap_or(true),
    )
}

/// Concierge / agent can push a generative UI surface described as JSON.
#[tauri::command]
pub fn push_generative_ui(
    app: tauri::AppHandle,
    title: String,
    root: serde_json::Value,
) -> Result<String, String> {
    use tauri::Manager;
    let id = format!("gen-{}", uuid::Uuid::new_v4());
    let _ = app.emit_all(
        "genui-push",
        serde_json::json!({ "id": &id, "title": title, "root": root }),
    );
    Ok(id)
}


// ── omniforge-fs plugin commands ────────────────────────────────────────

#[tauri::command]
pub fn fs_atomic_write(path: String, content: String) -> Result<(), String> {
    crate::plugins::fs_plugin::FsPlugin::atomic_write(path, content)
}

#[tauri::command]
pub fn fs_atomic_copy(from: String, to: String) -> Result<u64, String> {
    crate::plugins::fs_plugin::FsPlugin::atomic_copy(from, to)
}

#[tauri::command]
pub fn fs_cas(path: String, expected: Option<String>, content: String) -> Result<bool, String> {
    crate::plugins::fs_plugin::FsPlugin::cas(path, expected, content)
}

#[tauri::command]
pub fn fs_cleanup_orphans(dir: String) -> Result<usize, String> {
    crate::plugins::fs_plugin::FsPlugin::cleanup_orphans(dir)
}

#[tauri::command]
pub fn list_rust_plugins() -> Vec<crate::plugins::PluginMeta> {
    let mut reg = crate::plugins::PluginRegistry::new();
    reg.register(std::sync::Arc::new(crate::plugins::fs_plugin::FsPlugin));
    reg.list()
}

#[tauri::command]
pub fn cas_write_source(
    app: tauri::AppHandle,
    path: String,
    expected: Option<String>,
    content: String,
) -> Result<bool, String> {
    crate::self_edit::cas_write_source(&path, expected.as_deref(), &content, Some(&app))
}

#[tauri::command]
pub fn live_runtime_smoke_test() -> &'static str {
    "smoke-test-ok"
}
