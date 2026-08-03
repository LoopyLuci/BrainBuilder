#![allow(dead_code)]
//! ModelMistress connector — treat ModelMistress as an external HTTP service
//! and expose a thin command surface to the frontend. BrainBuilder does NOT
//! link against `model-mistress` directly; all interaction is over HTTP so
//! the two repos can evolve independently.

use serde::{Deserialize, Serialize};
use std::sync::Arc;
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
}
