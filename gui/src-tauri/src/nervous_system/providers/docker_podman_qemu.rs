use super::super::provider::SandboxProvider;
use super::super::types::{
    ProviderCapabilities, SandboxJob, SandboxSpec, Snapshot, JobMetrics, JobStatus, JobState,
    ResourceLimits,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Default)]
struct DockerState {
    jobs: Vec<SandboxJob>,
}

pub struct DockerProvider {
    state: Arc<RwLock<DockerState>>,
}

impl DockerProvider {
    pub fn new() -> Self {
        Self { state: Arc::new(RwLock::new(DockerState::default())) }
    }

    async fn run_docker(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("docker")
            .args(args)
            .output()
            .await
            .map_err(|e| format!("docker execution failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("docker error: {}", stderr));
        }

        String::from_utf8(output.stdout).map_err(|e| format!("docker output parse failed: {}", e))
    }

    async fn find_job(&self, job_id: &str) -> Option<SandboxJob> {
        let state = self.state.read().await;
        state.jobs.iter().find(|j| j.id == job_id).cloned()
    }
}

#[async_trait::async_trait]
impl SandboxProvider for DockerProvider {
    fn id(&self) -> &'static str { "docker" }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            name: "Docker".into(),
            version: "20.10+".into(),
            vm: false,
            container: true,
            process_jail: true,
            gpu: true,
            network_isolation: true,
            live_migration: false,
            snapshot: true,
            max_instances: Some(256),
        }
    }

    async fn submit(&self, spec: SandboxSpec) -> Result<SandboxJob, String> {
        let image = spec.image.clone().unwrap_or_else(|| "alpine:latest".to_string());
        let container_name = format!("luci-docker-{}", Uuid::new_v4());
        let runtime_id = format!("docker:{}", container_name);

        let mut docker_args: Vec<String> = vec![
            "run".into(), "-d".into(), "--name".into(), container_name.clone(),
        ];

        let res = &spec.resources;
        if let Some(cpu) = res.cpu_cores {
            docker_args.extend(["--cpus".into(), cpu.to_string()]);
        }
        if let Some(mem) = res.memory_mb {
            docker_args.extend(["-m".into(), format!("{}m", mem)]);
        }
        if let Some(pids) = res.pids {
            docker_args.extend(["--pids-limit".into(), pids.to_string()]);
        }
        if res.gpu {
            docker_args.extend(["--gpus".into(), "all".into()]);
        }

        for (key, value) in &spec.env {
            docker_args.extend(["-e".into(), format!("{}={}", key, value)]);
        }

        if spec.network {
            docker_args.extend(["--network".into(), "bridge".into()]);
        }

        for mount in &spec.mounts {
            let mode = if mount.read_only { "ro" } else { "rw" };
            docker_args.extend(["-v".into(), format!("{}:{}:{}", mount.host_path, mount.guest_path, mode)]);
        }

        for port in &spec.ports {
            docker_args.extend(["-p".into(), format!("{}:{}", port.host, port.guest)]);
        }

        for (key, value) in &spec.labels {
            docker_args.extend(["--label".into(), format!("{}={}", key, value)]);
        }

        docker_args.push(image);
        for arg in &spec.command {
            docker_args.push(arg.clone());
        }

        let args_ref: Vec<&str> = docker_args.iter().map(|s| s.as_str()).collect();
        let container_id_raw = self.run_docker(&args_ref).await?;
        let _container_id = container_id_raw.trim().to_string();

        let now = chrono::Utc::now();
        let job = SandboxJob {
            id: format!("docker-{}", Uuid::new_v4()),
            provider: self.id().into(),
            spec,
            status: JobStatus::Running,
            state: JobState::Active,
            exit_code: None,
            created_at: now,
            started_at: Some(now),
            finished_at: None,
            runtime_id: Some(runtime_id.clone()),
            endpoint: None,
            logs: Vec::new(),
            metrics: JobMetrics {
                cpu_secs: 0.0,
                memory_mb_peak: 0,
                net_rx_bytes: 0,
                net_tx_bytes: 0,
                disk_bytes_read: 0,
                disk_bytes_written: 0,
            },
        };

        self.state.write().await.jobs.push(job.clone());
        Ok(job)
    }

    async fn start(&self, job_id: &str) -> Result<(), String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        self.run_docker(&["start", &container]).await?;
        Ok(())
    }

    async fn stop(&self, job_id: &str) -> Result<(), String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let _ = self.run_docker(&["stop", &container]).await;
        Ok(())
    }

    async fn pause(&self, job_id: &str) -> Result<(), String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        self.run_docker(&["pause", &container]).await?;
        Ok(())
    }

    async fn resume(&self, job_id: &str) -> Result<(), String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        self.run_docker(&["unpause", &container]).await?;
        Ok(())
    }

    async fn wait(&self, job_id: &str) -> Result<SandboxJob, String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let _ = self.run_docker(&["wait", &container]).await;
        self.status(job_id).await
    }

    async fn logs(&self, job_id: &str, tail: Option<u32>) -> Result<Vec<String>, String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let mut args: Vec<String> = vec!["logs".into(), container.clone()];
        if let Some(n) = tail {
            args.extend(["--tail".into(), n.to_string()]);
        }
        let args_ref: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let output = self.run_docker(&args_ref).await?;
        Ok(output.lines().map(|s| s.to_string()).collect())
    }

    async fn metrics(&self, job_id: &str) -> Result<JobMetrics, String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let output = self.run_docker(&[
            "stats", "--no-stream", "--format", "{{.CPUPerc}}\t{{.MemUsage}}", &container,
        ]).await?;

        let parts: Vec<&str> = output.trim().split('\t').collect();
        let mut metrics = JobMetrics {
            cpu_secs: 0.0,
            memory_mb_peak: 0,
            net_rx_bytes: 0,
            net_tx_bytes: 0,
            disk_bytes_read: 0,
            disk_bytes_written: 0,
        };

        if parts.len() >= 2 {
            if let Some(cpu_str) = parts[0].strip_suffix('%') {
                if let Ok(cpu) = cpu_str.parse::<f64>() {
                    metrics.cpu_secs = cpu;
                }
            }
            if let Some(mem_str) = parts[1].strip_suffix("MiB") {
                if let Ok(mem) = mem_str.trim().parse::<u64>() {
                    metrics.memory_mb_peak = mem;
                }
            }
        }

        Ok(metrics)
    }

    async fn snapshot(&self, job_id: &str) -> Result<Snapshot, String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let snapshot_name = format!("luci-snap-{}", Uuid::new_v4());
        self.run_docker(&["commit", &container, &snapshot_name]).await?;

        Ok(Snapshot {
            id: snapshot_name.clone(),
            job_id: job.id.clone(),
            provider: self.id().into(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
            state: "docker-image".into(),
        })
    }

    async fn restore(&self, snapshot_id: &str) -> Result<SandboxJob, String> {
        let new_name = format!("luci-restored-{}", Uuid::new_v4());
        self.run_docker(&["run", "-d", "--name", &new_name, snapshot_id]).await?;

        let now = chrono::Utc::now();
        let job = SandboxJob {
            id: format!("docker-{}", Uuid::new_v4()),
            provider: self.id().into(),
            spec: SandboxSpec {
                provider: "docker".into(),
                image: Some(snapshot_id.to_string()),
                command: Vec::new(),
                env: Vec::new(),
                working_dir: None,
                resources: ResourceLimits {
                    cpu_cores: None,
                    memory_mb: None,
                    disk_mb: None,
                    pids: None,
                    ulimit_nofile: None,
                    gpu: false,
                },
                network: false,
                mounts: Vec::new(),
                ports: Vec::new(),
                labels: Vec::new(),
                annotations: Vec::new(),
            },
            status: JobStatus::Running,
            state: JobState::Active,
            exit_code: None,
            created_at: now,
            started_at: Some(now),
            finished_at: None,
            runtime_id: Some(format!("docker:{}", new_name)),
            endpoint: None,
            logs: Vec::new(),
            metrics: JobMetrics {
                cpu_secs: 0.0,
                memory_mb_peak: 0,
                net_rx_bytes: 0,
                net_tx_bytes: 0,
                disk_bytes_read: 0,
                disk_bytes_written: 0,
            },
        };

        self.state.write().await.jobs.push(job.clone());
        Ok(job)
    }

    async fn delete(&self, job_id: &str) -> Result<(), String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        let container = job.runtime_id.unwrap_or_default().strip_prefix("docker:").unwrap_or_default().to_string();
        let _ = self.run_docker(&["rm", "-f", &container]).await;
        let mut state = self.state.write().await;
        state.jobs.retain(|j| j.id != job_id);
        Ok(())
    }

    async fn list(&self) -> Result<Vec<SandboxJob>, String> {
        Ok(self.state.read().await.jobs.clone())
    }

    async fn status(&self, job_id: &str) -> Result<SandboxJob, String> {
        self.find_job(job_id).await.ok_or_else(|| format!("docker job not found: {}", job_id))
    }
}

pub struct PodmanProvider;

impl PodmanProvider {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl SandboxProvider for PodmanProvider {
    fn id(&self) -> &'static str { "podman" }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            name: "Podman".into(),
            version: "stub".into(),
            vm: true,
            container: true,
            process_jail: true,
            gpu: true,
            network_isolation: true,
            live_migration: false,
            snapshot: true,
            max_instances: Some(128),
        }
    }

    async fn submit(&self, _spec: SandboxSpec) -> Result<SandboxJob, String> { Err("podman provider not wired yet".into()) }
    async fn start(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn stop(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn pause(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn resume(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn wait(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
    async fn logs(&self, _id: &str, _tail: Option<u32>) -> Result<Vec<String>, String> { Ok(vec![]) }
    async fn metrics(&self, _id: &str) -> Result<JobMetrics, String> { Ok(JobMetrics { cpu_secs: 0.0, memory_mb_peak: 0, net_rx_bytes: 0, net_tx_bytes: 0, disk_bytes_read: 0, disk_bytes_written: 0 }) }
    async fn snapshot(&self, _id: &str) -> Result<Snapshot, String> { Err("not wired".into()) }
    async fn restore(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
    async fn delete(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn list(&self) -> Result<Vec<SandboxJob>, String> { Ok(vec![]) }
    async fn status(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
}

pub struct QemuProvider;

impl QemuProvider {
    pub fn new() -> Self { Self }
}

#[async_trait::async_trait]
impl SandboxProvider for QemuProvider {
    fn id(&self) -> &'static str { "qemu" }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            name: "QEMU/KVM".into(),
            version: "stub".into(),
            vm: true,
            container: false,
            process_jail: false,
            gpu: true,
            network_isolation: true,
            live_migration: true,
            snapshot: true,
            max_instances: Some(16),
        }
    }

    async fn submit(&self, _spec: SandboxSpec) -> Result<SandboxJob, String> { Err("qemu provider not wired yet".into()) }
    async fn start(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn stop(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn pause(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn resume(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn wait(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
    async fn logs(&self, _id: &str, _tail: Option<u32>) -> Result<Vec<String>, String> { Ok(vec![]) }
    async fn metrics(&self, _id: &str) -> Result<JobMetrics, String> { Ok(JobMetrics { cpu_secs: 0.0, memory_mb_peak: 0, net_rx_bytes: 0, net_tx_bytes: 0, disk_bytes_read: 0, disk_bytes_written: 0 }) }
    async fn snapshot(&self, _id: &str) -> Result<Snapshot, String> { Err("not wired".into()) }
    async fn restore(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
    async fn delete(&self, _id: &str) -> Result<(), String> { Ok(()) }
    async fn list(&self) -> Result<Vec<SandboxJob>, String> { Ok(vec![]) }
    async fn status(&self, _id: &str) -> Result<SandboxJob, String> { Err("not wired".into()) }
}
