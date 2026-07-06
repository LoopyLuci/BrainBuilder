//! Resolves a `"provider:model"` selector to a live [`LlmProvider`] + the bare
//! model id. This is the single seam the authoring and synthesis code go
//! through, so nothing downstream ever hardcodes Ollama-vs-OpenCode: the GUI
//! passes a selector string, the registry hands back something that speaks the
//! [`LlmProvider`] trait, and the model id to call it with.
//!
//! Providers are built lazily/per-call rather than held as long-lived state:
//! the OpenCode key can change at runtime (the user pastes it into the Models
//! panel), and both provider structs are cheap to construct.
use crate::interop::protocol::BrainBuilderError;
use crate::llm::provider::{LlmProvider, OllamaProvider, OpenCodeProvider};
use crate::Result;

/// How the OpenCode API key is sourced when a selector names the `opencode`
/// provider. The Tauri layer supplies the real keychain-backed lookup; tests
/// can inject a literal.
pub type KeyLookup = dyn Fn() -> Option<String> + Send + Sync;

pub struct ProviderRegistry {
    opencode_key: Box<KeyLookup>,
}

impl ProviderRegistry {
    /// `opencode_key` is consulted only when an `opencode:*` selector is
    /// resolved — so a user who never touches OpenCode never triggers a
    /// keychain read.
    pub fn new(opencode_key: Box<KeyLookup>) -> Self {
        Self { opencode_key }
    }

    /// The providers a GUI selector should offer, as `(id, display_name)`.
    /// Static: both providers always exist; whether they're *usable* is a
    /// runtime concern (Ollama running? OpenCode key set?) surfaced when the
    /// user actually picks one.
    pub fn available(&self) -> Vec<(String, String)> {
        vec![
            ("ollama".to_string(), "Ollama (local)".to_string()),
            ("opencode".to_string(), "OpenCode Go (hosted)".to_string()),
        ]
    }

    /// Split a `"provider:model"` selector and hand back the resolved provider
    /// plus the bare model id. A selector with no `provider:` prefix defaults
    /// to Ollama, preserving the pre-multi-provider behaviour of callers that
    /// still pass a bare model name.
    pub fn resolve(&self, selector: &str) -> Result<(Box<dyn LlmProvider>, String)> {
        let (provider_id, model) = match selector.split_once(':') {
            Some((p, m)) => (p, m),
            None => ("ollama", selector),
        };
        let provider: Box<dyn LlmProvider> = match provider_id {
            "ollama" => Box::new(OllamaProvider::new()),
            "opencode" => {
                let key = (self.opencode_key)().unwrap_or_default();
                Box::new(OpenCodeProvider::new(key))
            }
            other => {
                return Err(BrainBuilderError::ConfigError(format!(
                    "unknown LLM provider `{other}` in selector `{selector}` (known: ollama, opencode)"
                )))
            }
        };
        Ok((provider, model.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn registry() -> ProviderRegistry {
        ProviderRegistry::new(Box::new(|| Some("test-key".to_string())))
    }

    #[test]
    fn resolves_an_explicit_opencode_selector() {
        let (provider, model) = registry().resolve("opencode:opencode-go/glm-5.2").unwrap();
        assert_eq!(provider.id(), "opencode");
        assert_eq!(model, "opencode-go/glm-5.2");
    }

    #[test]
    fn resolves_an_explicit_ollama_selector() {
        let (provider, model) = registry().resolve("ollama:llama3").unwrap();
        assert_eq!(provider.id(), "ollama");
        assert_eq!(model, "llama3");
    }

    #[test]
    fn a_bare_model_defaults_to_ollama() {
        let (provider, model) = registry().resolve("llama3").unwrap();
        assert_eq!(provider.id(), "ollama");
        assert_eq!(model, "llama3");
    }

    #[test]
    fn an_unknown_provider_is_a_clear_error() {
        // `resolve` yields a boxed trait object that isn't `Debug`, so match
        // the error out explicitly rather than via `unwrap_err`.
        match registry().resolve("skynet:hal9000") {
            Ok(_) => panic!("an unknown provider should not resolve"),
            Err(e) => assert!(e.to_string().contains("skynet"), "error should name the bad provider: {e}"),
        }
    }

    #[test]
    fn available_lists_both_providers() {
        let ids: Vec<String> = registry().available().into_iter().map(|(id, _)| id).collect();
        assert!(ids.contains(&"ollama".to_string()));
        assert!(ids.contains(&"opencode".to_string()));
    }
}
