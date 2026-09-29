use crate::interop::protocol::BrainBuilderError;
use crate::runtime::nervous_system::system_resources::resolve_memory_limit_bytes;
use crate::runtime::nervous_system::{Capabilities, ComponentRuntime, Supervisor};
use crate::Result;
use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

/// Bridges to `languages/clojure/brainbuilder_config.clj` by shelling out to
/// a plain `java -cp <classpath> clojure.main bridge.clj` (rather than
/// embedding the JVM in-process via `jni`, which would need an embedded
/// `libjvm` bootstrap plus Clojure on the classpath managed at runtime — a
/// much bigger undertaking than a subprocess for a config DSL that's
/// evaluated occasionally, not on a hot path), spawned through the nervous
/// system's `Supervisor`: capped to reading `languages/clojure/` and the
/// resolved `~/.m2` classpath jars, no network, 15s wall-clock timeout (JVM
/// startup is slower than Racket's). See `languages/clojure/bridge.clj` for
/// the wire protocol (one line in, one line out).
pub struct ClojureBridge {
    caps: Capabilities,
}

impl Default for ClojureBridge {
    fn default() -> Self {
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        // Same auto-scaling as RacketEngine, but a JVM needs real headroom
        // for its own reservation (see FALLBACK_MEMORY_LIMIT_BYTES's history
        // in supervisor.rs) — higher floor and fraction than Racket's.
        let memory_limit = resolve_memory_limit_bytes(
            "BRAINBUILDER_SUBPROCESS_MEMORY_MB",
            0.35,
            2 * 1024 * 1024 * 1024,
            16 * 1024 * 1024 * 1024,
        );
        let mut caps = Capabilities::none()
            .allow_read(manifest_dir.join("../languages/clojure"))
            .with_timeout(Duration::from_secs(15))
            .with_memory_limit(memory_limit);
        if let Ok(m2) = dirs_home_m2() {
            caps = caps.allow_read(m2);
        }
        Self { caps }
    }
}

impl ComponentRuntime for ClojureBridge {
    fn name(&self) -> &str {
        "clojure"
    }
    fn capabilities(&self) -> &Capabilities {
        &self.caps
    }
}

impl ClojureBridge {
    /// `form` is a full Clojure form as text, e.g.
    /// `(defgraph my-graph :loss "mse")`. Returns its printed (`pr-str`)
    /// result, or an error if evaluation failed.
    pub fn eval_config(&self, form: &str) -> Result<String> {
        let classpath = clojure_classpath()?;
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let bridge_path = manifest_dir.join("../languages/clojure/bridge.clj");
        let config_path = manifest_dir.join("../languages/clojure/brainbuilder_config.clj");

        let mut cmd = Command::new("java");
        cmd.arg(format!("-Dbrainbuilder.config.path={}", config_path.display()))
            .arg("-cp")
            .arg(&classpath)
            .arg("clojure.main")
            .arg(&bridge_path);

        let stdin_data = format!("{form}\n");
        let output = Supervisor::run_checked_named("clojure", &mut cmd, &self.caps, &[&bridge_path, &config_path], Some(&stdin_data))
            .map_err(|e| BrainBuilderError::Clojure(e.to_string()))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let line = stdout.lines().next().unwrap_or("").trim();

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(BrainBuilderError::Clojure(format!(
                "java subprocess exited with {}: {}",
                output.status, stderr
            )));
        }

        if let Some(message) = line.strip_prefix("ERROR: ") {
            return Err(BrainBuilderError::Clojure(message.to_string()));
        }

        Ok(line.to_string())
    }
}

/// Finds the newest jar under a `~/.m2/repository/org/clojure/<artifact>`
/// directory tree. These land there once anything (e.g. `lein repl`, `lein
/// deps`) has resolved Clojure's own dependencies at least once; there's no
/// portable way to embed/vendor a JVM classpath without a real dependency
/// manager doing that resolution first.
fn newest_jar_under(dir: PathBuf) -> Option<PathBuf> {
    let mut jars = Vec::new();
    fn walk(dir: &std::path::Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().map_or(false, |e| e == "jar") {
                out.push(path);
            }
        }
    }
    walk(&dir, &mut jars);
    jars.sort();
    jars.pop()
}

fn clojure_classpath() -> Result<String> {
    let m2 = dirs_home_m2()?;
    let clojure_repo = m2.join("org/clojure");

    let clojure_jar = newest_jar_under(clojure_repo.join("clojure")).ok_or_else(|| {
        BrainBuilderError::Clojure(
            "no Clojure jar found under ~/.m2/repository/org/clojure/clojure — run `lein repl` \
             once in languages/clojure/ to let Leiningen resolve it"
                .into(),
        )
    })?;
    let spec_jar = newest_jar_under(clojure_repo.join("spec.alpha")).ok_or_else(|| {
        BrainBuilderError::Clojure("no spec.alpha jar found under ~/.m2/repository/org/clojure".into())
    })?;
    let core_specs_jar =
        newest_jar_under(clojure_repo.join("core.specs.alpha")).ok_or_else(|| {
            BrainBuilderError::Clojure(
                "no core.specs.alpha jar found under ~/.m2/repository/org/clojure".into(),
            )
        })?;

    let sep = if cfg!(windows) { ";" } else { ":" };
    Ok(format!(
        "{}{sep}{}{sep}{}",
        clojure_jar.display(),
        spec_jar.display(),
        core_specs_jar.display()
    ))
}

fn dirs_home_m2() -> Result<PathBuf> {
    let home = std::env::var_os("USERPROFILE")
        .or_else(|| std::env::var_os("HOME"))
        .ok_or_else(|| BrainBuilderError::Clojure("could not determine home directory".into()))?;
    Ok(PathBuf::from(home).join(".m2/repository"))
}
