#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod concierge_bridge;
mod model_executor;
mod platform_host;
mod km_format;
mod export_bundle;
mod training_executor;
mod plugin_system;
mod dataset_tools;
mod multimodal_merger;
mod inference_sandbox;
mod rag;
mod vector_store;
mod self_edit;
mod atomic_fs;
mod plugins;
mod paths;

use concierge_bridge::*;
use concierge_core::{ConciergeAgent, OpenRouterClient};
use model_executor::ModelExecutor;
use platform_host::OmniForgeHost;
use std::sync::Arc;
use tracing_subscriber::EnvFilter;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("tokio runtime");

    // SQLite-backed host
    let host = rt
        .block_on(async {
            let path = crate::paths::sqlite_path("omniforge.db").expect("data dir");
            tracing::info!(?path, "OmniForge database");
            OmniForgeHost::new(&path.to_string_lossy())
        })
        .expect("Failed to open OmniForge database");
    let host = Arc::new(host);
    let host_for_agent = Arc::clone(&host);
    let host_for_setup = Arc::clone(&host);

    let llm_client = Arc::new(OpenRouterClient::free());
    let executor = Arc::new(ModelExecutor::new());

    let agent = rt
        .block_on(async {
            struct SharedHost(Arc<OmniForgeHost>);
            #[async_trait::async_trait]
            impl concierge_core::tools::implementations::PlatformHost for SharedHost {
                async fn search_models(
                    &self,
                    q: &str,
                    m: Option<&str>,
                    a: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.search_models(q, m, a).await
                }
                async fn import_model(
                    &self,
                    p: &str,
                    f: Option<&str>,
                    n: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.import_model(p, f, n).await
                }
                async fn list_models(
                    &self,
                    limit: usize,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.list_models(limit).await
                }
                async fn add_node(
                    &self,
                    nt: &str,
                    mid: Option<&str>,
                    label: Option<&str>,
                    x: f64,
                    y: f64,
                    cfg: Option<serde_json::Value>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.add_node(nt, mid, label, x, y, cfg).await
                }
                async fn connect_nodes(
                    &self,
                    s: &str,
                    ss: &str,
                    t: &str,
                    ts: &str,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.connect_nodes(s, ss, t, ts).await
                }
                async fn create_dataset(
                    &self,
                    name: &str,
                    sources: &[String],
                    labels: Option<&[String]>,
                    format: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.create_dataset(name, sources, labels, format).await
                }
                async fn run_training(
                    &self,
                    bm: &str,
                    ds: &str,
                    recipe: &str,
                    out: Option<&str>,
                    epochs: Option<i64>,
                    lr: Option<f64>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.run_training(bm, ds, recipe, out, epochs, lr).await
                }
                async fn create_knowledge_module(
                    &self,
                    bm: &str,
                    ap: &str,
                    meta: Option<&str>,
                    name: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.create_knowledge_module(bm, ap, meta, name).await
                }
                async fn merge_models(
                    &self,
                    ids: &[String],
                    strategy: &str,
                    weights: Option<&[f64]>,
                    out: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.merge_models(ids, strategy, weights, out).await
                }
                async fn inspect_node(
                    &self,
                    id: &str,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.inspect_node(id).await
                }
                async fn execute_graph(
                    &self,
                    inputs: &serde_json::Value,
                    timeout: Option<u64>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.execute_graph(inputs, timeout).await
                }
                async fn compare_outputs(
                    &self,
                    a: &str,
                    b: &str,
                    input: &str,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.compare_outputs(a, b, input).await
                }
                async fn search_docs(
                    &self,
                    q: &str,
                    limit: usize,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.search_docs(q, limit).await
                }
                async fn write_plugin(
                    &self,
                    desc: &str,
                    lang: &str,
                    name: Option<&str>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.write_plugin(desc, lang, name).await
                }
                async fn modify_graph(
                    &self,
                    action: &str,
                    target: &str,
                    payload: Option<&serde_json::Value>,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.modify_graph(action, target, payload).await
                }
                async fn get_platform_status(
                    &self,
                ) -> Result<String, concierge_core::ConciergeError> {
                    self.0.get_platform_status().await
                }
            }

            ConciergeAgent::with_host(
                llm_client,
                &crate::paths::sqlite_path("concierge.db").expect("data dir").to_string_lossy(),
                Box::new(SharedHost(host_for_agent)),
            )
            .await
        })
        .expect("Failed to initialize Concierge");

    tauri::Builder::default()
        .manage(AppState {
            concierge: tokio::sync::Mutex::new(agent),
            host,
            executor,
        })
        .setup(move |app| {
            let handle = app.handle();
            let h = host_for_setup.clone();
            tauri::async_runtime::spawn(async move {
                h.set_app_handle(handle).await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            concierge_chat,
            get_canvas,
            import_model,
            add_node,
            connect_nodes,
            inspect_node,
            execute_graph,
            run_training,
            get_platform_status,
            list_models,
            search_models,
            load_onnx_model,
            infer_onnx,
            start_gguf_model,
            stop_model,
            list_running_models,
            list_gguf_models,
            // Extended capabilities
            start_training,
            create_km,
            export_model_bundle,
            get_plugins,
            run_plugin_tool,
            augment_dataset,
            process_multimodal,
            execute_canvas_graph,
            rag_retrieve,
            rag_index_km,
            hybrid_search,
            vector_upsert,
            list_editable_sources,
            read_source_file,
            atomic_write_source,
            push_generative_ui,
            fs_atomic_write,
            fs_atomic_copy,
            fs_cas,
            fs_cleanup_orphans,
            list_rust_plugins,
            cas_write_source,
            live_runtime_smoke_test,
        ])
        .run(tauri::generate_context!())
        .expect("error while running OmniForge");
}
