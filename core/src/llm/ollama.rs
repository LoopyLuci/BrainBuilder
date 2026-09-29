// Real local LLM authoring: a plain HTTP client against Ollama's documented
// local REST API (http://localhost:11434, no cloud dependency, no API key —
// the actual "local-first" requirement). Ollama itself is what handles GGUF
// model loading/quantized inference correctly; this client only ever talks
// to it over loopback HTTP, matching the same delegation approach `models`
// uses for GGUF weight files.
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use serde::{Deserialize, Serialize};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

pub struct OllamaClient {
    base_url: String,
    client: reqwest::Client,
}

#[derive(Serialize)]
struct GenerateRequest<'a> {
    model: &'a str,
    prompt: &'a str,
    stream: bool,
    /// Ollama's real, documented "constrain output to valid JSON" mode —
    /// far more reliable than asking nicely in the prompt for a model this
    /// small to actually emit parseable JSON every time.
    format: &'a str,
    system: &'a str,
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

#[derive(Deserialize)]
struct TagsResponse {
    models: Vec<TagEntry>,
}

#[derive(Deserialize)]
struct TagEntry {
    name: String,
}

impl OllamaClient {
    pub fn new() -> Self {
        Self::with_base_url(DEFAULT_BASE_URL.to_string())
    }

    pub fn with_base_url(base_url: String) -> Self {
        Self {
            base_url,
            // Local-loopback only — a stuck/nonexistent Ollama server should
            // fail fast with a clear error, not hang the GUI indefinitely.
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(120))
                .build()
                .expect("reqwest client construction failed"),
        }
    }

    /// Real reachability check + the actual list of models this local
    /// Ollama install has pulled — used to give a clear, specific error
    /// ("Ollama isn't running" vs. "you haven't pulled any models yet")
    /// instead of a generic connection failure.
    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let resp = self.client.get(&url).send().await.map_err(|e| {
            BrainBuilderError::ConfigError(format!(
                "couldn't reach Ollama at {} — is it running? (`ollama serve`): {e}",
                self.base_url
            ))
        })?;
        let parsed: TagsResponse = resp
            .json()
            .await
            .map_err(|e| BrainBuilderError::ConfigError(format!("unexpected response from Ollama's /api/tags: {e}")))?;
        Ok(parsed.models.into_iter().map(|m| m.name).collect())
    }

    /// One real, non-streaming generation call, constrained to JSON output.
    /// Returns the raw response text — callers that expect a specific JSON
    /// shape (e.g. a BBIR graph) parse and validate it themselves rather
    /// than trusting the model's output blindly.
    pub async fn generate_json(&self, model: &str, system: &str, prompt: &str) -> Result<String> {
        let url = format!("{}/api/generate", self.base_url);
        let req = GenerateRequest { model, prompt, stream: false, format: "json", system };
        let resp = self.client.post(&url).json(&req).send().await.map_err(|e| {
            BrainBuilderError::ConfigError(format!(
                "couldn't reach Ollama at {} — is it running? (`ollama serve`): {e}",
                self.base_url
            ))
        })?;
        if !resp.status().is_success() {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            return Err(BrainBuilderError::ConfigError(format!(
                "Ollama returned {status} — is `{model}` pulled? (`ollama pull {model}`). Body: {body}"
            )));
        }
        let parsed: GenerateResponse = resp
            .json()
            .await
            .map_err(|e| BrainBuilderError::ConfigError(format!("unexpected response shape from Ollama's /api/generate: {e}")))?;
        Ok(parsed.response)
    }
}

impl Default for OllamaClient {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Proves the client fails clearly and quickly against a real closed
    /// port, rather than hanging — this is the one thing genuinely
    /// verifiable about the Ollama integration in an environment where a
    /// live Ollama server can't be bound (see `graph_author.rs`'s module
    /// doc for why). A real reachable server is exercised manually by a
    /// developer with Ollama actually running; this test guards the failure
    /// path instead.
    #[tokio::test]
    async fn reports_a_clear_error_when_ollama_is_unreachable() {
        // Port 1 is a real, always-unbound-by-anything-real port on any
        // machine (privileged, reserved) — guaranteed connection-refused
        // rather than accidentally hitting a real service.
        let client = OllamaClient::with_base_url("http://localhost:1".to_string());
        let result = client.list_models().await;
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("Ollama"), "error should clearly name Ollama as the thing that failed: {msg}");
    }
}
