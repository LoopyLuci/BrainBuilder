// Real local inference serving: "Export checkpoint…" hands a file to
// something else; this instead starts an actual HTTP server, in-process,
// that answers real `POST /predict` requests against the currently trained
// checkpoint — so any other program (curl, a script, a web app) can get
// predictions from BrainBuilder over the network without exporting or
// reimplementing anything. Unlike `observer_server.rs`'s LAN-facing Cluster
// viewer, this binds loopback-only (127.0.0.1) by default: an
// unauthenticated inference endpoint is a much more sensitive surface than a
// read-only status page, so it never listens beyond this machine.
use axum::extract::State as AxumState;
use axum::http::StatusCode;
use axum::response::Json;
use axum::routing::post;
use axum::Router;
use brainbuilder_core::bbir::BBIRGraph;
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener};
use tauri::Manager;

#[derive(serde::Deserialize)]
struct PredictRequest {
    graph_json: String,
    dataset_path: String,
    rows: usize,
}

#[derive(serde::Serialize)]
struct PredictResponseItem {
    shape: Vec<i64>,
    values: Vec<f32>,
}

type HandlerError = (StatusCode, String);

async fn predict_handler(
    AxumState(app): AxumState<tauri::AppHandle>,
    Json(req): Json<PredictRequest>,
) -> Result<Json<Vec<PredictResponseItem>>, HandlerError> {
    let graph: BBIRGraph = serde_json::from_str(&req.graph_json)
        .map_err(|e| (StatusCode::BAD_REQUEST, format!("invalid graph_json: {e}")))?;
    let batch = brainbuilder_core::data::source::load_batch(&req.dataset_path, req.rows)
        .await
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    let state = app.state::<crate::AppState>();
    let orchestrator = state.orchestrator.lock().await;
    let tensors = orchestrator
        .predict(graph, batch)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let items = tensors
        .iter()
        .map(|t| {
            brainbuilder_core::interop::dlpack_support::tensor_to_vec_f32(t)
                .map(|(shape, values)| PredictResponseItem { shape, values })
                .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))
        })
        .collect::<Result<Vec<_>, HandlerError>>()?;
    Ok(Json(items))
}

/// Starts the real local predict server (loopback-only, OS-assigned port) and
/// returns the URL to POST real predict requests to, plus a shutdown sender
/// that stops it gracefully — the caller owns the sender, so at most one
/// server per app run is a policy enforced by whoever holds it (see
/// `AppState::predict_server` in `main.rs`), not by this function.
pub fn spawn(app: tauri::AppHandle) -> (String, tokio::sync::oneshot::Sender<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("failed to bind predict server");
    listener.set_nonblocking(true).expect("failed to set predict listener non-blocking");
    let port = listener.local_addr().expect("bound listener has a local addr").port();

    let router = Router::new().route("/predict", post(predict_handler)).with_state(app);
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();

    tauri::async_runtime::spawn(async move {
        let listener = tokio::net::TcpListener::from_std(listener).expect("failed to adopt predict listener into tokio");
        let server = axum::serve(listener, router).with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        });
        if let Err(e) = server.await {
            log::error!("predict HTTP server stopped: {e}");
        }
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port);
    (format!("http://{addr}"), shutdown_tx)
}
