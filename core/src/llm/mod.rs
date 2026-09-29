//! Local-first LLM-assisted graph authoring, now provider-agnostic. A
//! [`provider::LlmProvider`] abstracts the backend — the always-local Ollama
//! server (`ollama.rs`) or OpenCode Go's hosted OpenAI-compatible endpoint
//! (`provider.rs`) — and [`registry::ProviderRegistry`] resolves a
//! `"provider:model"` selector to one. The prompt-building/parse-and-validate
//! pipeline (`graph_author.rs`) turns a provider's output into a real,
//! validated BBIR graph and is identical across providers.
pub mod graph_author;
pub mod ollama;
pub mod provider;
pub mod registry;

pub use graph_author::{
    build_system_prompt, generate_graph_from_description, generate_graph_with_provider,
    parse_and_validate,
};
pub use ollama::OllamaClient;
pub use provider::{LlmProvider, OllamaProvider, OpenCodeProvider, OPENCODE_MODELS};
pub use registry::ProviderRegistry;
