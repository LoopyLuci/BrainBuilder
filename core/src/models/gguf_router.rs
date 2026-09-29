// Real GGUF routing via Ollama, not a from-scratch quantized-tensor engine.
// GGUF (llama.cpp's format) packs weights in a dozen+ quantization schemes
// (Q4_K_M, Q5_1, Q8_0, ...); correctly dequantizing and running inference on
// all of them is what an entire mature project (llama.cpp, which Ollama
// embeds) exists to do. Reimplementing that here would either be a much
// larger undertaking than this session's scope, or a shallow subset that
// silently mishandles real production quantization formats — neither is
// "enterprise grade." Delegating to a real local `ollama` install (which the
// nervous system's `Supervisor` already knows how to run subprocesses
// under) is the sound engineering choice, and it's genuinely offline: once
// a model is registered with `ollama create`, no network access is needed.
use crate::interop::protocol::BrainBuilderError;
use crate::llm::OllamaClient;
use crate::runtime::nervous_system::{Capabilities, ComponentRuntime, Supervisor};
use crate::Result;
use std::path::Path;
use std::process::Command;
use std::time::Duration;

pub struct GgufRouter {
    /// Always empty — unlike every other `ComponentRuntime` here
    /// (Python/Racket/Clojure), whose touched directories are fixed and
    /// known upfront at construction, the GGUF path is arbitrary and
    /// user-chosen per call. `register_local_gguf` builds its own real,
    /// call-scoped grant instead (see its doc comment); this field exists
    /// only to satisfy the `ComponentRuntime` trait's accessor.
    caps: Capabilities,
}

impl ComponentRuntime for GgufRouter {
    fn name(&self) -> &str {
        "gguf-router"
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
}

impl Default for GgufRouter {
    fn default() -> Self {
        Self { caps: Capabilities::none() }
    }
}

impl GgufRouter {
    /// Registers a local `.gguf` file with Ollama under `model_name`, via a
    /// real (auto-generated) Modelfile and a real `ollama create` subprocess
    /// call — the documented, supported way to point Ollama at an arbitrary
    /// local GGUF weight file rather than one of its hub-pulled models.
    /// Idempotent: re-registering the same name overwrites the prior
    /// registration, matching `ollama create`'s own real behavior. The
    /// capability grant is built fresh per call, scoped to exactly the two
    /// directories this specific call touches (the GGUF file's own
    /// directory, and the OS temp dir for the generated Modelfile) — the
    /// GGUF path is arbitrary and user-chosen, not known upfront the way
    /// Python/Racket/Clojure's fixed component directories are.
    pub fn register_local_gguf(&self, gguf_path: &Path, model_name: &str) -> Result<()> {
        if !gguf_path.is_file() {
            return Err(BrainBuilderError::ConfigError(format!("GGUF file not found: {}", gguf_path.display())));
        }
        let gguf_dir = gguf_path.parent().ok_or_else(|| {
            BrainBuilderError::ConfigError(format!("GGUF path `{}` has no parent directory", gguf_path.display()))
        })?;
        let temp_dir = std::env::temp_dir();
        let caps = Capabilities::none().allow_read(gguf_dir).allow_read(&temp_dir).with_timeout(Duration::from_secs(300));

        let modelfile_path = temp_dir.join(format!("brainbuilder_modelfile_{model_name}"));
        let modelfile_contents = format!("FROM {}\n", gguf_path.display());
        std::fs::write(&modelfile_path, &modelfile_contents).map_err(|e| {
            BrainBuilderError::ConfigError(format!("failed to write Ollama Modelfile: {e}"))
        })?;

        let mut cmd = Command::new("ollama");
        cmd.args(["create", model_name, "-f"]).arg(&modelfile_path);
        let output = Supervisor::run_checked(&mut cmd, &caps, &[modelfile_path.as_path(), gguf_path], None)
            .map_err(|e| BrainBuilderError::ConfigError(format!("`ollama create` failed to run: {e}")))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrainBuilderError::ConfigError(format!(
                "`ollama create {model_name}` exited with {}: {stderr}",
                output.status
            )));
        }
        Ok(())
    }

    /// Chats with an already-registered (via `register_local_gguf`, or
    /// already `ollama pull`ed) model, over the same local HTTP API the
    /// LLM-authoring feature uses.
    pub async fn chat(&self, model_name: &str, prompt: &str) -> Result<String> {
        let client = OllamaClient::new();
        client.generate_json(model_name, "", prompt).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_local_gguf_errors_clearly_on_a_missing_file() {
        let router = GgufRouter::default();
        let err = router.register_local_gguf(Path::new("/definitely/not/a/real/path.gguf"), "test-model").unwrap_err();
        assert!(err.to_string().contains("not found"));
    }
}
