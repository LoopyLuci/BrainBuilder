//! OpenRouter client with automatic fallback across free models.
//! Also works with any OpenAI-compatible endpoint (Ollama, Together, Groq, …).

use async_trait::async_trait;
use reqwest::Client;
use tracing::{debug, info, warn};

use crate::error::ConciergeError;
use crate::llm_client::{LlmClient, LlmRequest, LlmResponse};

/// Free models on OpenRouter that can be used without a paid key.
pub const FREE_MODELS: &[&str] = &[
    "mistralai/mistral-7b-instruct:free",
    "google/gemma-2-9b-it:free",
    "meta-llama/llama-3.2-3b-instruct:free",
    "microsoft/phi-3-mini-128k-instruct:free",
    "qwen/qwen-2-7b-instruct:free",
    "openchat/openchat-7b:free",
];

pub struct OpenRouterClient {
    client: Client,
    api_key: Option<String>,
    base_url: String,
    http_referer: Option<String>,
    site_title: Option<String>,
    current_model: String,
    fallback_models: Vec<String>,
}

impl OpenRouterClient {
    /// Free-tier client with automatic model fallback on rate-limit / server errors.
    pub fn free() -> Self {
        let models: Vec<String> = FREE_MODELS.iter().map(|s| (*s).to_string()).collect();
        let first = models.first().cloned().unwrap_or_default();
        Self {
            client: Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("Failed to build HTTP client"),
            api_key: std::env::var("OPENROUTER_API_KEY").ok(),
            base_url: "https://openrouter.ai/api/v1".to_string(),
            http_referer: Some("https://omniforge.ai".to_string()),
            site_title: Some("OmniForge Concierge".to_string()),
            current_model: first,
            fallback_models: models,
        }
    }

    /// Legacy constructor (api_key optional).
    pub fn new(api_key: Option<String>) -> Self {
        let mut c = Self::free();
        if api_key.is_some() {
            c.api_key = api_key;
        }
        c
    }

    pub fn with_model(mut self, model: impl Into<String>, fallbacks: Vec<String>) -> Self {
        self.current_model = model.into();
        if !fallbacks.is_empty() {
            self.fallback_models = fallbacks;
        }
        self
    }

    pub fn with_base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    pub fn with_referer(mut self, referer: impl Into<String>) -> Self {
        self.http_referer = Some(referer.into());
        self
    }

    pub fn with_site_title(mut self, title: impl Into<String>) -> Self {
        self.site_title = Some(title.into());
        self
    }

    pub fn ollama(base: &str) -> Self {
        Self::free().with_base_url(format!("{}/v1", base.trim_end_matches('/')))
    }

    pub fn current_model(&self) -> &str {
        &self.current_model
    }

    fn apply_headers(&self, mut req: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        if let Some(ref key) = self.api_key {
            req = req.header("Authorization", format!("Bearer {}", key));
        }
        if let Some(ref referer) = self.http_referer {
            req = req.header("HTTP-Referer", referer);
        }
        if let Some(ref title) = self.site_title {
            req = req.header("X-Title", title);
        }
        req
    }

    async fn try_chat(&self, request: &LlmRequest) -> Result<LlmResponse, ConciergeError> {
        let url = format!("{}/chat/completions", self.base_url.trim_end_matches('/'));
        debug!(model = %request.model, messages = request.messages.len(), "Sending LLM request");

        let mut req = self.client.post(&url).json(request);
        req = self.apply_headers(req);

        let resp = req.send().await.map_err(|e| {
            ConciergeError::LlmClient(format!("Request failed: {e}"))
        })?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            return Err(ConciergeError::LlmClient(format!(
                "Provider error {status}: {body}"
            )));
        }

        let llm_response: LlmResponse = resp.json().await.map_err(|e| {
            ConciergeError::LlmClient(format!("Failed to parse response JSON: {e}"))
        })?;

        if llm_response.choices.is_empty() {
            return Err(ConciergeError::LlmClient("Empty choices array".into()));
        }
        info!(finish_reason = ?llm_response.choices[0].finish_reason, "LLM response received");
        Ok(llm_response)
    }
}

#[async_trait]
impl LlmClient for OpenRouterClient {
    async fn chat(&self, mut request: LlmRequest) -> Result<LlmResponse, ConciergeError> {
        let models: Vec<String> = {
            let mut m = vec![self.current_model.clone()];
            m.extend(self.fallback_models.iter().cloned());
            m.dedup();
            m
        };

        let mut last_err = None;
        for model in &models {
            request.model = model.clone();
            match self.try_chat(&request).await {
                Ok(resp) => return Ok(resp),
                Err(e) => {
                    let status_hint = e.to_string();
                    let retryable = status_hint.contains("429")
                        || status_hint.contains("503")
                        || status_hint.contains("500")
                        || status_hint.contains("502")
                        || status_hint.contains("Request failed");
                    warn!(model = %model, error = %e, retryable, "Model attempt failed");
                    last_err = Some(e);
                    if !retryable {
                        break;
                    }
                }
            }
        }
        Err(last_err.unwrap_or_else(|| {
            ConciergeError::LlmClient("All models exhausted, no response".into())
        }))
    }
}
