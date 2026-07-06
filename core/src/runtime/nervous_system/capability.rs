use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Deny-by-default capability grant for one component runtime instance.
/// Nothing is implicitly allowed: a runtime can only touch the paths listed
/// here, and only for as long as `timeout` permits.
#[derive(Debug, Clone, Default)]
pub struct Capabilities {
    read_paths: Vec<PathBuf>,
    network: bool,
    timeout: Option<Duration>,
    memory_limit_bytes: Option<u64>,
}

impl Capabilities {
    /// No filesystem, no network, no time limit, no memory ceiling — the
    /// safe empty default.
    pub fn none() -> Self {
        Self::default()
    }

    pub fn allow_read(mut self, path: impl Into<PathBuf>) -> Self {
        self.read_paths.push(path.into());
        self
    }

    pub fn allow_network(mut self) -> Self {
        self.network = true;
        self
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    pub fn with_memory_limit(mut self, bytes: u64) -> Self {
        self.memory_limit_bytes = Some(bytes);
        self
    }

    pub fn timeout(&self) -> Option<Duration> {
        self.timeout
    }

    pub fn network_allowed(&self) -> bool {
        self.network
    }

    pub fn memory_limit_bytes(&self) -> Option<u64> {
        self.memory_limit_bytes
    }

    /// Errors unless `path` is inside (or equal to) one of the granted
    /// `read_paths`. Canonicalizes both sides so `..`/symlinks can't be used
    /// to escape the grant.
    pub fn check_read(&self, path: &Path) -> Result<()> {
        let target = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        for allowed in &self.read_paths {
            let allowed_canon = allowed.canonicalize().unwrap_or_else(|_| allowed.clone());
            if target.starts_with(&allowed_canon) {
                return Ok(());
            }
        }
        Err(BrainBuilderError::ConfigError(format!(
            "capability denied: `{}` is not under any granted read path",
            path.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn denies_paths_outside_the_grant() {
        let caps = Capabilities::none().allow_read(std::env::temp_dir());
        assert!(caps.check_read(Path::new("C:/definitely/not/granted")).is_err());
    }

    #[test]
    fn allows_paths_inside_the_grant() {
        let dir = std::env::temp_dir();
        let caps = Capabilities::none().allow_read(&dir);
        assert!(caps.check_read(&dir).is_ok());
    }

    #[test]
    fn network_and_timeout_default_to_denied_and_unbounded_off() {
        let caps = Capabilities::none();
        assert!(!caps.network_allowed());
        assert!(caps.timeout().is_none());
    }
}
