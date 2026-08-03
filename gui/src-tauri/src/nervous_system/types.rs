use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxSpec {
    pub provider: String,
    pub image: Option<String>,
    pub command: Vec<String>,
    pub env: Vec<(String, String)>,
    pub working_dir: Option<String>,
    pub resources: ResourceLimits,
    pub network: bool,
    pub mounts: Vec<Mount>,
    pub ports: Vec<Port>,
    pub labels: Vec<(String, String)>,
    pub annotations: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    pub cpu_cores: Option<f64>,
    pub memory_mb: Option<u64>,
    pub disk_mb: Option<u64>,
    pub pids: Option<u64>,
    pub ulimit_nofile: Option<u64>,
    pub gpu: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mount {
    pub host_path: String,
    pub guest_path: String,
    pub read_only: bool,
    pub propagation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Port {
    pub host: u16,
    pub guest: u16,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxJob {
    pub id: String,
    pub provider: String,
    pub spec: SandboxSpec,
    pub status: JobStatus,
    pub state: JobState,
    pub exit_code: Option<i32>,
    pub created_at: DateTime<Utc>,
    pub started_at: Option<DateTime<Utc>>,
    pub finished_at: Option<DateTime<Utc>>,
    pub runtime_id: Option<String>,
    pub endpoint: Option<String>,
    pub logs: Vec<String>,
    pub metrics: JobMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobMetrics {
    pub cpu_secs: f64,
    pub memory_mb_peak: u64,
    pub net_rx_bytes: u64,
    pub net_tx_bytes: u64,
    pub disk_bytes_read: u64,
    pub disk_bytes_written: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobStatus {
    Queued,
    Starting,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Pending,
    Active,
    Exited,
    Paused,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub id: String,
    pub name: String,
    pub version: String,
    pub vm: bool,
    pub container: bool,
    pub process_jail: bool,
    pub gpu: bool,
    pub network_isolation: bool,
    pub live_migration: bool,
    pub snapshot: bool,
    pub max_instances: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Snapshot {
    pub id: String,
    pub job_id: String,
    pub provider: String,
    pub created_at: DateTime<Utc>,
    pub size_bytes: u64,
    pub state: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NervousSystemStatus {
    pub providers: Vec<ProviderCapabilities>,
    pub jobs: Vec<SandboxJob>,
    pub total_jobs: u64,
    pub active_jobs: u64,
    pub failed_jobs_24h: u64,
}
