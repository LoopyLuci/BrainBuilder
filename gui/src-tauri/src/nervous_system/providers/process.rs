use crate::nervous_system::provider::SandboxProvider;
use crate::nervous_system::types::{
    ProviderCapabilities, SandboxJob, SandboxSpec, Snapshot, JobMetrics, JobStatus,
};

pub struct ProcessJailProvider;

#[async_trait::async_trait]
impl SandboxProvider for ProcessJailProvider {
    fn id(&self) -> &'static str { "process_jail" }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            name: "Process Jail".into(),
            version: "0.1.0".into(),
            vm: false,
            container: false,
            process_jail: true,
            gpu: false,
            network_isolation: false,
            live_migration: false,
            snapshot: false,
            max_instances: Some(64),
        }
    }

    async fn submit(&self, spec: SandboxSpec) -> Result<SandboxJob, String> {
        let runtime_id = format!("pid-{}", uuid::Uuid::new_v4());
        let now = chrono::Utc::now();
        Ok(SandboxJob {
            id: format!("{}-{}", self.id(), uuid::Uuid::new_v4()),
            provider: self.id().into(),
            spec,
            status: JobStatus::Queued,
            state: crate::nervous_system::types::JobState::Pending,
            exit_code: None,
            created_at: now,
            started_at: None,
            finished_at: None,
            runtime_id: Some(runtime_id.clone()),
            endpoint: None,
            logs: Vec::new(),
            metrics: JobMetrics { cpu_secs: 0.0, memory_mb_peak: 0, net_rx_bytes: 0, net_tx_bytes: 0, disk_bytes_read: 0, disk_bytes_written: 0 },
        })
    }

    async fn start(&self, _job_id: &str) -> Result<(), String> { Ok(()) }
    async fn stop(&self, _job_id: &str) -> Result<(), String> { Ok(()) }
    async fn pause(&self, _job_id: &str) -> Result<(), String> { Ok(()) }
    async fn resume(&self, _job_id: &str) -> Result<(), String> { Ok(()) }
    async fn wait(&self, _job_id: &str) -> Result<SandboxJob, String> { Err("process_jail wait not implemented".into()) }
    async fn logs(&self, job_id: &str, _tail: Option<u32>) -> Result<Vec<String>, String> { Ok(vec![format!("process_jail logs for {}", job_id)]) }
    async fn metrics(&self, _job_id: &str) -> Result<JobMetrics, String> { Ok(JobMetrics { cpu_secs: 0.0, memory_mb_peak: 0, net_rx_bytes: 0, net_tx_bytes: 0, disk_bytes_read: 0, disk_bytes_written: 0 }) }
    async fn snapshot(&self, _job_id: &str) -> Result<Snapshot, String> { Err("process_jail snapshot not supported".into()) }
    async fn restore(&self, snapshot_id: &str) -> Result<SandboxJob, String> { Err(format!("process_jail restore not supported for {}", snapshot_id)) }
    async fn delete(&self, _job_id: &str) -> Result<(), String> { Ok(()) }
    async fn list(&self) -> Result<Vec<SandboxJob>, String> { Ok(Vec::new()) }
    async fn status(&self, job_id: &str) -> Result<SandboxJob, String> { Err(format!("process_jail status not implemented for {}", job_id)) }
}
