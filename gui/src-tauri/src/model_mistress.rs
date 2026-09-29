#![allow(dead_code)]
//! ModelMistress connector — treats ModelMistress as an external HTTP service
//! and exposes a thin command surface to the frontend. BrainBuilder does NOT
//! link against `model-mistress` directly; all interaction is over HTTP so
//! the two repos can evolve independently.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tauri::Manager;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct ModelMistressBridge {
    pub base_url: Arc<Mutex<String>>,
    http: reqwest::Client,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmModelInfo {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub owned_by: String,
    pub size_bytes: Option<u64>,
    pub format: Option<String>,
    pub quantization: Option<String>,
    pub context_length: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmChatMessage {
    pub role: String,
    pub content: Option<String>,
    pub tool_calls: Option<serde_json::Value>,
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmChatRequest {
    pub model: String,
    pub messages: Vec<MmChatMessage>,
    pub stream: Option<bool>,
    pub temperature: Option<f32>,
    pub top_p: Option<f32>,
    pub max_tokens: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmCompletionChoice {
    pub index: u32,
    pub message: MmChatMessage,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmUsage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmChatResponse {
    pub id: String,
    pub object: String,
    pub created: i64,
    pub model: String,
    pub choices: Vec<MmCompletionChoice>,
    pub usage: MmUsage,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmHealth {
    pub status: String,
    pub version: String,
    pub timestamp: String,
    pub ollama: String,
    pub local_models_loaded: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmStreamChunk {
    pub id: Option<String>,
    pub object: Option<String>,
    pub created: Option<i64>,
    pub model: Option<String>,
    pub choices: Option<Vec<MmStreamChoice>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmStreamChoice {
    pub index: Option<u32>,
    pub delta: Option<MmStreamDelta>,
    pub finish_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MmStreamDelta {
    pub role: Option<String>,
    pub content: Option<String>,
}

impl ModelMistressBridge {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: Arc::new(Mutex::new(base_url.into())),
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .unwrap_or_default(),
        }
    }

    pub async fn set_base_url(&self, url: impl Into<String>) {
        *self.base_url.lock().await = url.into();
    }

    pub async fn health(&self) -> Result<MmHealth, String> {
        let url = format!("{}/health", self.base_url.lock().await.clone());
        self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn list_models(&self) -> Result<Vec<MmModelInfo>, String> {
        let url = format!("{}/v1/models", self.base_url.lock().await.clone());
        let body = self.http
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json::<serde_json::Value>()
            .await
            .map_err(|e| e.to_string())?;
        let data = body.get("data").cloned().unwrap_or(serde_json::Value::Null);
        let models = serde_json::from_value(data).map_err(|e| e.to_string())?;
        Ok(models)
    }

    pub async fn chat(&self, req: MmChatRequest) -> Result<MmChatResponse, String> {
        let url = format!("{}/v1/chat/completions", self.base_url.lock().await.clone());
        self.http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())
    }

    pub async fn stream_chat(&self, req: MmChatRequest, app: &tauri::AppHandle, node_id: &str) -> Result<String, String> {
        let url = format!("{}/v1/chat/completions", self.base_url.lock().await.clone());
        let mut full = String::new();
        let resp = self.http
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| e.to_string())?;
        if !resp.status().is_success() {
            return Err(format!("stream failed: HTTP {}", resp.status()));
        }
        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        let text = String::from_utf8_lossy(&bytes).to_string();
        full = text.clone();

        // Best-effort chunk emission: frontend can also parse raw SSE text
        let _ = app.emit_all("model-mistress-token", text);
        Ok(full)
    }
}

// --- Tauri command wrappers ---
// These wrap the bridge so the frontend can call them via `invoke()`.
// State follows the app-wide convention: `State<'_, crate::AppState>` and
// then `state.ext.lock().await` for shared services.

#[tauri::command]
pub async fn model_mistress_set_base_url(
    state: tauri::State<'_, crate::AppState>,
    url: String,
) -> Result<(), String> {
    let ext = state.ext.lock().await;
    ext.model_mistress.set_base_url(url).await;
    Ok(())
}

#[tauri::command]
pub async fn model_mistress_health(
    state: tauri::State<'_, crate::AppState>,
) -> Result<MmHealth, String> {
    let ext = state.ext.lock().await;
    ext.model_mistress.health().await
}

#[tauri::command]
pub async fn model_mistress_list_models(
    state: tauri::State<'_, crate::AppState>,
) -> Result<Vec<MmModelInfo>, String> {
    let ext = state.ext.lock().await;
    ext.model_mistress.list_models().await
}

#[tauri::command]
pub async fn model_mistress_chat(
    state: tauri::State<'_, crate::AppState>,
    model: String,
    messages: Vec<MmChatMessage>,
    stream: Option<bool>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
) -> Result<MmChatResponse, String> {
    let ext = state.ext.lock().await;
    let resp = ext.model_mistress.chat(MmChatRequest {
        model,
        messages,
        stream,
        temperature,
        top_p,
        max_tokens,
    }).await?;
    Ok(resp)
}

#[tauri::command]
pub async fn model_mistress_stream_tokens(
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
    node_id: String,
    model: String,
    messages: Vec<MmChatMessage>,
    prompt: Option<String>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
) -> Result<String, String> {
    let ext = state.ext.lock().await;
    let mut final_messages = messages;
    if let Some(p) = prompt {
        final_messages.push(MmChatMessage { role: "user".into(), content: Some(p), tool_calls: None, tool_call_id: None });
    }
    ext.model_mistress.stream_chat(
        MmChatRequest {
            model,
            messages: final_messages,
            stream: Some(true),
            temperature,
            top_p,
            max_tokens,
        },
        &app,
        &node_id,
    ).await
}

#[tauri::command]
pub async fn model_mistress_chat_completion(
    state: tauri::State<'_, crate::AppState>,
    app: tauri::AppHandle,
    model: String,
    messages: Vec<MmChatMessage>,
    prompt: Option<String>,
    stream: Option<bool>,
    temperature: Option<f32>,
    top_p: Option<f32>,
    max_tokens: Option<u32>,
) -> Result<String, String> {
    let ext = state.ext.lock().await;
    let mut final_messages = messages;
    if let Some(p) = prompt {
        final_messages.push(MmChatMessage { role: "user".into(), content: Some(p), tool_calls: None, tool_call_id: None });
    }
    let node_id = format!("model-mistress-{}", uuid::Uuid::new_v4());
    if stream.unwrap_or(false) {
        ext.model_mistress.stream_chat(
            MmChatRequest {
                model,
                messages: final_messages,
                stream: Some(true),
                temperature,
                top_p,
                max_tokens,
            },
            &app,
            &node_id,
        ).await
    } else {
        let resp = ext.model_mistress.chat(MmChatRequest {
            model,
            messages: final_messages,
            stream: Some(false),
            temperature,
            top_p,
            max_tokens,
        }).await?;
        Ok(resp.choices.into_iter().next().and_then(|c| c.message.content).unwrap_or_default())
    }
}

pub mod commands {
    pub use super::{
        model_mistress_chat,
        model_mistress_chat_completion,
        model_mistress_health,
        model_mistress_list_models,
        model_mistress_set_base_url,
        model_mistress_stream_tokens,
    };
}
