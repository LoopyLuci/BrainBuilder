//! Provider-agnostic LLM access. BrainBuilder's authoring/synthesis paths used
//! to assume Ollama specifically; this trait lets any backend that can (a) turn
//! a system+user prompt into JSON text and (b) list its available models stand
//! in interchangeably. Two implementations ship:
//!
//! - [`OllamaProvider`] — the always-local, no-key default (wraps the existing
//!   [`crate::llm::ollama::OllamaClient`], loopback HTTP to `ollama serve`).
//! - [`OpenCodeProvider`] — OpenCode Go's hosted, OpenAI-compatible chat
//!   completions endpoint (a bearer API key, a large menu of open models).
//!
//! Callers never hardcode a provider: they resolve one from a
//! [`crate::llm::registry::ProviderRegistry`] by a `"provider:model"` selector,
//! so the same graph-authoring / component-synthesis code drives Ollama or
//! OpenCode identically.
use crate::interop::protocol::BrainBuilderError;
use crate::llm::ollama::OllamaClient;
use crate::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};

/// One interchangeable LLM backend. `generate_json` must return the model's
/// raw textual output (the caller parses + validates it — a provider never
/// gets to be trusted about structure); `list_models` is a reachability check
/// that doubles as the model menu shown in the GUI.
#[async_trait]
pub trait LlmProvider: Send + Sync {
    /// Stable identifier used as the left half of a `"provider:model"`
    /// selector (e.g. `"ollama"`, `"opencode"`).
    fn id(&self) -> &str;

    /// Human-facing name for the provider selector in the GUI.
    fn display_name(&self) -> &str;

    /// The models this provider can serve right now. For a local provider this
    /// is a live reachability probe; for a hosted one it's the published menu.
    async fn list_models(&self) -> Result<Vec<String>>;

    /// One non-streaming generation, asking for JSON output. Returns raw text.
    async fn generate_json(&self, model: &str, system: &str, prompt: &str) -> Result<String>;
}

/// The local-first default: delegates straight to [`OllamaClient`].
pub struct OllamaProvider {
    client: OllamaClient,
}

impl OllamaProvider {
    pub fn new() -> Self {
        Self { client: OllamaClient::new() }
    }

    pub fn with_client(client: OllamaClient) -> Self {
        Self { client }
    }
}

impl Default for OllamaProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn id(&self) -> &str {
        "ollama"
    }

    fn display_name(&self) -> &str {
        "Ollama (local)"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        self.client.list_models().await
    }

    async fn generate_json(&self, model: &str, system: &str, prompt: &str) -> Result<String> {
        self.client.generate_json(model, system, prompt).await
    }
}

/// OpenCode Go's hosted, OpenAI-compatible endpoint. The published model menu
/// is static (it changes rarely and only via OpenCode's roster), so we don't
/// depend on a `/models` round-trip to populate the selector — but a real key
/// is required for `generate_json`, and the error paths say so specifically.
pub struct OpenCodeProvider {
    base_url: String,
    api_key: String,
    client: reqwest::Client,
}

/// OpenCode Go's published open-model roster (July 2026), namespaced the way
/// their API expects (`opencode-go/<model>`). Kept as a constant so the GUI
/// selector works offline / before any network call; the list is cheap to
/// extend as OpenCode adds models.
pub const OPENCODE_MODELS: &[&str] = &[
    "opencode-go/glm-5.2",
    "opencode-go/glm-5.1",
    "opencode-go/kimi-k2.7-code",
    "opencode-go/kimi-k2.6",
    "opencode-go/mimo-v2.5-pro",
    "opencode-go/mimo-v2.5",
    "opencode-go/qwen3.7-max",
    "opencode-go/qwen3.7-plus",
    "opencode-go/qwen3.6-plus",
    "opencode-go/minimax-m2.7",
    "opencode-go/minimax-m3",
    "opencode-go/deepseek-v4-pro",
    "opencode-go/deepseek-v4-flash",
];

const OPENCODE_DEFAULT_BASE_URL: &str = "https://opencode.ai/zen/go/v1";

#[derive(Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    stream: bool,
    /// OpenAI-compatible "force valid JSON" mode — the hosted analogue of
    /// Ollama's `format: "json"`, so the same downstream parse/validate step
    /// works regardless of provider.
    response_format: ResponseFormat,
}

#[derive(Serialize)]
struct ResponseFormat {
    #[serde(rename = "type")]
    kind: &'static str,
}

#[derive(Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Deserialize)]
struct ChatChoice {
    message: ChatResponseMessage,
}

#[derive(Deserialize)]
struct ChatResponseMessage {
    content: String,
}

impl OpenCodeProvider {
    pub fn new(api_key: String) -> Self {
        Self::with_base_url(api_key, OPENCODE_DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(api_key: String, base_url: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client construction failed"),
        }
    }
}

#[async_trait]
impl LlmProvider for OpenCodeProvider {
    fn id(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "OpenCode Go (hosted)"
    }

    async fn list_models(&self) -> Result<Vec<String>> {
        // Static roster — no network needed to populate the selector, and no
        // key required just to see what's on offer.
        Ok(OPENCODE_MODELS.iter().map(|m| m.to_string()).collect())
    }

    async fn generate_json(&self, model: &str, system: &str, prompt: &str) -> Result<String> {
        if self.api_key.trim().is_empty() {
            return Err(BrainBuilderError::ConfigError(
                "OpenCode API key not set — add it in the Models panel (stored in your OS keychain, never on disk)".to_string(),
            ));
        }
        let url = format!("{}/chat/completions", self.base_url);
        let req = ChatRequest {
            model,
            messages: vec![
                ChatMessage { role: "system", content: system },
                ChatMessage { role: "user", content: prompt },
            ],
            stream: false,
            response_format: ResponseFormat { kind: "json_object" },
        };
        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&req)
            .send()
            .await
            .map_err(|e| {
                BrainBuilderError::ConfigError(format!("couldn't reach OpenCode at {}: {e}", self.base_url))
            })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainBuilderError::ConfigError(format!(
                "OpenCode returned {status} for `{model}` — check your API key and that the model id is valid. Body: {body}"
            )));
        }
        let parsed: ChatResponse = resp.json().await.map_err(|e| {
            BrainBuilderError::ConfigError(format!("unexpected response shape from OpenCode's chat completions: {e}"))
        })?;
        parsed
            .choices
            .into_iter()
            .next()
            .map(|c| c.message.content)
            .ok_or_else(|| BrainBuilderError::ConfigError("OpenCode returned no choices".to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_roster_is_namespaced() {
        assert!(OPENCODE_MODELS.iter().all(|m| m.starts_with("opencode-go/")));
        assert!(!OPENCODE_MODELS.is_empty());
    }

    #[tokio::test]
    async fn opencode_refuses_to_generate_without_a_key() {
        let provider = OpenCodeProvider::new(String::new());
        let err = provider.generate_json("opencode-go/glm-5.2", "sys", "hi").await.unwrap_err();
        assert!(err.to_string().contains("API key"), "should name the missing key: {err}");
    }

    #[tokio::test]
    async fn opencode_lists_its_static_roster_without_a_key() {
        let provider = OpenCodeProvider::new(String::new());
        let models = provider.list_models().await.expect("static roster needs no network");
        assert!(models.iter().any(|m| m.contains("glm")));
    }

    #[tokio::test]
    async fn opencode_reports_a_clear_error_against_an_unreachable_host() {
        let provider = OpenCodeProvider::with_base_url("key".to_string(), "http://localhost:1/v1".to_string());
        let err = provider.generate_json("m", "s", "p").await.unwrap_err();
        assert!(err.to_string().contains("OpenCode"), "error should name OpenCode: {err}");
    }

    #[test]
    fn ollama_provider_reports_its_identity() {
        let p = OllamaProvider::new();
        assert_eq!(p.id(), "ollama");
        assert_eq!(p.display_name(), "Ollama (local)");
    }
}
