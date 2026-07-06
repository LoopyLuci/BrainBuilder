//! Local-first LLM-assisted graph authoring: a plain HTTP client to a local
//! Ollama server (`ollama.rs`, no cloud dependency, no API key) and the
//! prompt-building/parse-and-validate pipeline that turns its output into a
//! real, validated BBIR graph (`graph_author.rs`).
pub mod graph_author;
pub mod ollama;

pub use graph_author::{build_system_prompt, generate_graph_from_description, parse_and_validate};
pub use ollama::OllamaClient;
