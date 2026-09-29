use async_trait::async_trait;
use crate::nervous_system::types::{
    ProviderCapabilities, SandboxJob, SandboxSpec, Snapshot, JobMetrics,
};

#[async_trait]
pub trait SandboxProvider: Send + Sync {
    fn id(&self) -> &'static str;
    async fn capabilities(&self) -> ProviderCapabilities;
    async fn submit(&self, spec: SandboxSpec) -> Result<SandboxJob, String>;
    async fn start(&self, job_id: &str) -> Result<(), String>;
    async fn stop(&self, job_id: &str) -> Result<(), String>;
    async fn pause(&self, job_id: &str) -> Result<(), String>;
    async fn resume(&self, job_id: &str) -> Result<(), String>;
    async fn wait(&self, job_id: &str) -> Result<SandboxJob, String>;
    async fn logs(&self, job_id: &str, tail: Option<u32>) -> Result<Vec<String>, String>;
    async fn metrics(&self, job_id: &str) -> Result<JobMetrics, String>;
    async fn snapshot(&self, job_id: &str) -> Result<Snapshot, String>;
    async fn restore(&self, snapshot_id: &str) -> Result<SandboxJob, String>;
    async fn delete(&self, job_id: &str) -> Result<(), String>;
    async fn list(&self) -> Result<Vec<SandboxJob>, String>;
    async fn status(&self, job_id: &str) -> Result<SandboxJob, String>;
}
