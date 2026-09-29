use crate::interop::protocol::BrainBuilderError;
use crate::runtime::nervous_system::system_resources::resolve_memory_limit_bytes;
use crate::runtime::nervous_system::{Capabilities, ComponentRuntime, Supervisor};
use std::process::Command;
use std::time::Duration;

/// Bridges to `languages/racket/symbolic_ad.rkt` by shelling out to `racket`
/// as a subprocess, spawned through the nervous system's `Supervisor`: capped
/// to reading only `languages/racket/`, no network, 10s wall-clock timeout.
/// Protocol: two lines on stdin (a racket-readable expression, then the
/// variable to differentiate with respect to), one line of output — either
/// the resulting expression, or `ERROR: <message>`. See
/// `languages/racket/bridge.rkt`.
pub struct RacketEngine {
    caps: Capabilities,
}

impl Default for RacketEngine {
    fn default() -> Self {
        let languages_dir =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../languages/racket");
        // Auto-scales with real system RAM (25% of total, floor 512MiB,
        // ceiling 16GiB) — override with BRAINBUILDER_SUBPROCESS_MEMORY_MB.
        // A fixed constant here would be needlessly tight on a 64GiB
        // desktop and dangerously loose on an 8GiB laptop.
        let memory_limit = resolve_memory_limit_bytes(
            "BRAINBUILDER_SUBPROCESS_MEMORY_MB",
            0.25,
            512 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
        );
        Self {
            caps: Capabilities::none()
                .allow_read(languages_dir)
                .with_timeout(Duration::from_secs(10))
                .with_memory_limit(memory_limit),
        }
    }
}

impl ComponentRuntime for RacketEngine {
    fn name(&self) -> &str {
        "racket"
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
}

impl RacketEngine {
    /// `expr` and `var` are both plain strings, e.g. `expr = "(* x x)"`,
    /// `var = "x"`. Returns the symbolically-differentiated expression as
    /// Racket's printed form, e.g. `"(+ (* x 1) (* x 1))"`.
    pub fn eval(&self, expr: &str, var: &str) -> Result<String, BrainBuilderError> {
        let bridge_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../languages/racket/bridge.rkt");

        let mut cmd = Command::new("racket");
        cmd.arg(&bridge_path);
        let stdin_data = format!("{expr}\n{var}\n");

        let output = Supervisor::run_checked_named("racket", &mut cmd, &self.caps, &[&bridge_path], Some(&stdin_data))
            .map_err(|e| BrainBuilderError::Racket(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next().unwrap_or("").trim();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrainBuilderError::Racket(format!(
                "racket subprocess exited with {}: {}",
                output.status, stderr
            )));
        }

        if let Some(message) = line.strip_prefix("ERROR: ") {
            return Err(BrainBuilderError::Racket(message.to_string()));
        }

        Ok(line.to_string())
    }
}
