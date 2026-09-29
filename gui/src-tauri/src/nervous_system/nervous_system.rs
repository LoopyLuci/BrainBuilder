use std::sync::Arc;
use tokio::sync::RwLock;
use crate::nervous_system::provider::SandboxProvider;
use crate::nervous_system::types::{
    ProviderCapabilities, SandboxJob, SandboxSpec, Snapshot, JobMetrics,
};

#[derive(Default)]
pub struct NervousSystem {
    providers: Vec<Arc<dyn SandboxProvider>>,
    jobs: Arc<RwLock<Vec<SandboxJob>>>,
}

impl NervousSystem {
    pub fn new() -> Self {
        Self { providers: Vec::new(), jobs: Arc::new(RwLock::new(Vec::new())) }
    }

    pub fn register(&mut self, provider: Arc<dyn SandboxProvider>) {
        self.providers.push(provider);
    }

    pub async fn capabilities(&self) -> Vec<ProviderCapabilities> {
        let mut out = Vec::new();
        for p in &self.providers {
            let c = p.capabilities().await;
            out.push(c);
        }
        out
    }

    pub async fn submit(&self, provider_id: &str, spec: SandboxSpec) -> Result<SandboxJob, String> {
        let provider = self.providers.iter().find(|p| p.id() == provider_id).ok_or_else(|| format!("provider not found: {}", provider_id))?;
        let mut job = provider.submit(spec).await?;
        job.id = format!("{}-{}", provider.id(), uuid::Uuid::new_v4());
        self.jobs.write().await.push(job.clone());
        Ok(job)
    }

    pub async fn start(&self, job_id: &str) -> Result<(), String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.start(job_id).await
    }

    pub async fn stop(&self, job_id: &str) -> Result<(), String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.stop(job_id).await
    }

    pub async fn pause(&self, job_id: &str) -> Result<(), String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.pause(job_id).await
    }

    pub async fn resume(&self, job_id: &str) -> Result<(), String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.resume(job_id).await
    }

    pub async fn wait(&self, job_id: &str) -> Result<SandboxJob, String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        let updated = provider.wait(job_id).await?;
        self.upsert_job(updated).await;
        self.get_job(job_id).await.ok_or_else(|| "job not found".into())
    }

    pub async fn logs(&self, job_id: &str, tail: Option<u32>) -> Result<Vec<String>, String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.logs(job_id, tail).await
    }

    pub async fn metrics(&self, job_id: &str) -> Result<JobMetrics, String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.metrics(job_id).await
    }

    pub async fn snapshot(&self, job_id: &str) -> Result<Snapshot, String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.snapshot(job_id).await
    }

    pub async fn restore(&self, snapshot_id: &str) -> Result<SandboxJob, String> {
        for p in &self.providers {
            if let Ok(job) = p.restore(snapshot_id).await {
                self.jobs.write().await.push(job.clone());
                return Ok(job);
            }
        }
        Err("no provider could restore snapshot".into())
    }

    pub async fn delete(&self, job_id: &str) -> Result<(), String> {
        let job = self.jobs.read().await.iter().find(|j| j.id == job_id).cloned().ok_or("job not found")?;
        let provider = self.providers.iter().find(|p| p.id() == &job.provider).ok_or("provider not found")?;
        provider.delete(job_id).await
    }

    pub async fn list(&self) -> Vec<SandboxJob> {
        self.jobs.read().await.clone()
    }

    async fn upsert_job(&self, updated: SandboxJob) {
        let mut jobs = self.jobs.write().await;
        if let Some(pos) = jobs.iter().position(|j| j.id == updated.id) {
            jobs[pos] = updated;
        } else {
            jobs.push(updated);
        }
    }

    async fn get_job(&self, id: &str) -> Option<SandboxJob> {
        self.jobs.read().await.iter().find(|j| j.id == id).cloned()
    }
}
