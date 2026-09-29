//! Local model loading + routing: discovering models already cached on this
//! machine (`discovery.rs`), loading real weights for formats BrainBuilder
//! can parse itself (`safetensors_loader.rs`), and routing formats better
//! served by a dedicated, battle-tested local runtime (GGUF -> Ollama,
//! `llm::ollama`; ONNX -> `onnx_loader.rs`'s `ort`-backed session) rather
//! than reimplementing quantized-tensor or graph-execution engines that
//! entire projects (llama.cpp, ONNX Runtime) exist specifically to do well.
pub mod discovery;
pub mod gguf_router;
pub mod safetensors_loader;

#[cfg(feature = "onnx")]
pub mod onnx_loader;

pub use discovery::{default_hf_cache_dir, scan_directory_for_models, scan_local_models, LocalModel, ModelFormat};
pub use gguf_router::GgufRouter;
