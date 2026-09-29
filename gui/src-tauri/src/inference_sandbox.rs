//! Canvas graph execution with **streaming** inference outputs.
//!
//! Events emitted to the frontend:
//! - `inference-start`     { graphNodeCount }
//! - `inference-node-start`{ nodeId, nodeType }
//! - `inference-token`     { nodeId, token, index }   ← token-by-token for GGUF
//! - `inference-output`    { nodeId, output }         ← final per-node payload
//! - `inference-error`     { nodeId, error }
//! - `inference-complete`  { outputs }

use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::Arc;
use tauri::Manager;
use tracing::{info, warn};

use crate::model_executor::ModelExecutor;
use crate::platform_host::OmniForgeHost;

pub async fn execute_graph(
    graph: &Value,
    host: &Arc<OmniForgeHost>,
    executor: &Arc<ModelExecutor>,
    app: &tauri::AppHandle,
) -> Result<Value, String> {
    let nodes = graph
        .get("nodes")
        .and_then(|n| n.as_array())
        .cloned()
        .unwrap_or_default();

    let _ = app.emit_all(
        "inference-start",
        json!({ "graphNodeCount": nodes.len() }),
    );

    let mut outputs: HashMap<String, Value> = HashMap::new();
    info!(count = nodes.len(), "Streaming graph execution");

    for node in &nodes {
        let id = node
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();
        let data = node.get("data").cloned().unwrap_or(Value::Null);
        let node_type = data
            .get("nodeType")
            .or_else(|| node.get("type"))
            .and_then(|v| v.as_str())
            .unwrap_or("model")
            .to_string();
        let model_id = data
            .get("modelId")
            .and_then(|v| v.as_str())
            .map(String::from);
        let input_text = data
            .get("input")
            .or_else(|| data.get("prompt"))
            .and_then(|v| v.as_str())
            .unwrap_or("Hello from OmniForge")
            .to_string();

        let _ = app.emit_all(
            "inference-node-start",
            json!({ "nodeId": &id, "nodeType": &node_type }),
        );

        let result = match node_type.as_str() {
            "model" => {
                run_model_node_streaming(executor, host, app, &id, model_id.as_deref(), &input_text)
                    .await
            }
            "adapter" => Ok(json!({
                "status": "ok",
                "note": "adapter applied in pipeline context",
                "modelId": model_id
            })),
            "prompt" => Ok(json!({ "prompt": input_text })),
            "output" => Ok(json!({
                "forwarded": outputs.values().last().cloned().unwrap_or(Value::Null)
            })),
            "rag" => {
                // RAG node: retrieve from attached KM corpus then pass context upstream
                let km_path = data
                    .get("kmPath")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                match crate::rag::retrieve(km_path, &input_text, 4).await {
                    Ok(ctx) => Ok(json!({
                        "context": ctx.chunks,
                        "scores": ctx.scores,
                        "query": input_text
                    })),
                    Err(e) => Err(e),
                }
            }
            other => Ok(json!({ "skipped": other })),
        };

        match result {
            Ok(payload) => {
                let _ = app.emit_all(
                    "inference-output",
                    json!({ "nodeId": &id, "output": &payload }),
                );
                outputs.insert(id, payload);
            }
            Err(e) => {
                warn!(node = %id, error = %e, "node failed");
                let err_payload = json!({ "error": e });
                let _ = app.emit_all(
                    "inference-error",
                    json!({ "nodeId": &id, "error": err_payload }),
                );
                let _ = app.emit_all(
                    "inference-output",
                    json!({ "nodeId": &id, "output": &err_payload }),
                );
                outputs.insert(id, err_payload);
            }
        }
    }

    let final_out = json!({ "outputs": outputs });
    let _ = app.emit_all("inference-complete", &final_out);
    Ok(final_out)
}

async fn run_model_node_streaming(
    executor: &Arc<ModelExecutor>,
    host: &Arc<OmniForgeHost>,
    app: &tauri::AppHandle,
    node_id: &str,
    model_id: Option<&str>,
    prompt: &str,
) -> Result<Value, String> {
    let path = resolve_model_path(host, model_id).await?;

    if let Some(ref p) = path {
        if p.ends_with(".onnx") {
            return run_onnx(executor, p, prompt).await;
        }
        if p.ends_with(".gguf") {
            return run_gguf_streaming(executor, app, node_id, p, prompt).await;
        }
    }

    // No model path – stream a demo response token-by-token so the UI path is exercised
    stream_demo_tokens(app, node_id, prompt).await;
    Ok(json!({
        "backend": "demo-stream",
        "prompt": prompt,
        "modelId": model_id,
        "note": "No resolvable model – streamed demo tokens"
    }))
}

async fn resolve_model_path(
    host: &Arc<OmniForgeHost>,
    model_id: Option<&str>,
) -> Result<Option<String>, String> {
    let Some(mid) = model_id else {
        return Ok(None);
    };
    let list = host.list_models(200).await.map_err(|e| e.to_string())?;
    let models: Vec<Value> = serde_json::from_str(&list).unwrap_or_default();
    Ok(models
        .iter()
        .find(|m| m.get("id").and_then(|v| v.as_str()) == Some(mid))
        .and_then(|m| m.get("path").and_then(|v| v.as_str()).map(String::from)))
}

async fn run_onnx(
    executor: &Arc<ModelExecutor>,
    path: &str,
    prompt: &str,
) -> Result<Value, String> {
    let sid = executor.load_onnx(path).await?;
    let inputs = serde_json::json!({
        "input": vec![0.0; 8],
        "__shapes__": { "input": [1, 8] }
    });
    match executor.infer_onnx(&sid, inputs).await {
        Ok(r) => Ok(json!({
            "backend": r.backend,
            "prompt": prompt,
            "outputs": r.outputs
        })),
        Err(e) => Ok(json!({
            "backend": "onnx",
            "prompt": prompt,
            "note": e,
            "path": path
        })),
    }
}

/// Stream GGUF completion: prefer SSE/stream endpoint, fall back to full completion
/// then emit tokens client-side for consistent UI behaviour.
async fn run_gguf_streaming(
    executor: &Arc<ModelExecutor>,
    app: &tauri::AppHandle,
    node_id: &str,
    path: &str,
    prompt: &str,
) -> Result<Value, String> {
    let running = executor.list_gguf().await;
    let info = running
        .into_iter()
        .find(|i| i.status == "running")
        .ok_or_else(|| {
            format!("No running GGUF server for {path}. Call start_gguf_model first.")
        })?;

    let body = json!({
        "prompt": prompt,
        "n_predict": 128,
        "stream": true,
        "temperature": 0.7
    });

    // Try streaming endpoint first
    match stream_gguf_sse(&info.endpoint, &body, app, node_id).await {
        Ok(full_text) => {
            return Ok(json!({
                "backend": "gguf-stream",
                "endpoint": info.endpoint,
                "text": full_text
            }));
        }
        Err(e) => {
            warn!(error = %e, "SSE stream failed – falling back to non-stream completion");
        }
    }

    // Non-stream fallback: request full completion, then emit tokens
    let body = json!({
        "prompt": prompt,
        "n_predict": 128,
        "stream": false
    });
    let resp = http_post_json(&format!("{}/completion", info.endpoint), &body).await?;
    let text = resp
        .get("content")
        .or_else(|| resp.get("text"))
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    emit_tokens(app, node_id, &text).await;

    Ok(json!({
        "backend": "gguf",
        "endpoint": info.endpoint,
        "text": text,
        "raw": resp
    }))
}

/// Consume llama.cpp-style SSE (`data: {...}`) and emit `inference-token` events.
async fn stream_gguf_sse(
    endpoint: &str,
    body: &Value,
    app: &tauri::AppHandle,
    node_id: &str,
) -> Result<String, String> {
    let url = format!("{endpoint}/completion");
    let body = body.clone();
    let node_id = node_id.to_string();
    let app = app.clone();

    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("HTTP {}", resp.status()));
        }

        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(resp);
        let mut full = String::new();
        let mut idx = 0u64;

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            let data = if let Some(rest) = line.strip_prefix("data: ") {
                rest.trim()
            } else if line.starts_with('{') {
                line.trim()
            } else {
                continue;
            };
            if data == "[DONE]" {
                break;
            }
            if let Ok(v) = serde_json::from_str::<Value>(data) {
                let token = v
                    .get("content")
                    .or_else(|| v.get("token"))
                    .and_then(|t| t.as_str())
                    .unwrap_or("");
                if !token.is_empty() {
                    full.push_str(token);
                    let _ = app.emit_all(
                        "inference-token",
                        json!({
                            "nodeId": &node_id,
                            "token": token,
                            "index": idx
                        }),
                    );
                    idx += 1;
                }
                if v.get("stop").and_then(|s| s.as_bool()) == Some(true) {
                    break;
                }
            }
        }
        Ok(full)
    })
    .await
    .map_err(|e| e.to_string())?
}

async fn emit_tokens(app: &tauri::AppHandle, node_id: &str, text: &str) {
    // Approximate tokens by whitespace for non-stream backends
    for (index, token) in text.split_inclusive(char::is_whitespace).enumerate() {
        let _ = app.emit_all(
            "inference-token",
            json!({ "nodeId": node_id, "token": token, "index": index }),
        );
        tokio::time::sleep(std::time::Duration::from_millis(12)).await;
    }
}

async fn stream_demo_tokens(app: &tauri::AppHandle, node_id: &str, prompt: &str) {
    let reply = format!(
        "OmniForge demo response for: \"{}\". Streaming works – connect a GGUF server for live tokens.",
        prompt.chars().take(80).collect::<String>()
    );
    emit_tokens(app, node_id, &reply).await;
}

async fn http_post_json(url: &str, body: &Value) -> Result<Value, String> {
    let url = url.to_string();
    let body = body.clone();
    tokio::task::spawn_blocking(move || {
        let client = reqwest::blocking::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .map_err(|e| e.to_string())?;
        let resp = client
            .post(&url)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?;
        let status = resp.status();
        let text = resp.text().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("HTTP {status}: {text}"));
        }
        serde_json::from_str(&text).or_else(|_| Ok(Value::String(text)))
    })
    .await
    .map_err(|e| e.to_string())?
}
