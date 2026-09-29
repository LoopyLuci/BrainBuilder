use super::Capabilities;

/// Uniform identity for any component executor plugged into the nervous
/// system (local subprocess today; a remote peer later — same trait, a
/// different `Supervisor` transport). `PythonBridge`/`RacketEngine`/
/// `ClojureBridge` each declare their own fixed capability grant.
pub trait ComponentRuntime: Send + Sync {
    fn name(&self) -> &str;
    fn capabilities(&self) -> &Capabilities;
}
