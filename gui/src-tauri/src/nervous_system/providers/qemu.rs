use super::super::provider::SandboxProvider;
use super::super::types::{
    ProviderCapabilities, SandboxJob, SandboxSpec, Snapshot, JobMetrics, JobStatus, JobState,
};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::process::Command;
use uuid::Uuid;

#[derive(Default)]
struct QemuState {
    jobs: Vec<SandboxJob>,
}

pub struct QemuProvider {
    state: Arc<RwLock<QemuState>>,
}

impl QemuProvider {
    pub fn new() -> Self {
        Self { state: Arc::new(RwLock::new(QemuState::default())) }
    }

    async fn run_qemu(&self, args: &[&str]) -> Result<String, String> {
        let output = Command::new("qemu-system-x86_64")
            .args(args)
            .output()
            .await
            .map_err(|e| format!("qemu execution failed: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("qemu error: {}", stderr));
        }

        String::from_utf8(output.stdout).map_err(|e| format!("qemu output parse failed: {}", e))
    }

    async fn find_job(&self, job_id: &str) -> Option<SandboxJob> {
        let state = self.state.read().await;
        state.jobs.iter().find(|j| j.id == job_id).cloned()
    }
}

#[async_trait::async_trait]
impl SandboxProvider for QemuProvider {
    fn id(&self) -> &'static str { "qemu" }

    async fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            name: "QEMU/KVM".into(),
            version: "6.2+".into(),
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

    async fn submit(&self, spec: SandboxSpec) -> Result<SandboxJob, String> {
        let image = spec.image.clone().unwrap_or_else(|| "cloud-init:default".to_string());
        let vm_name = format!("luci-qemu-{}", Uuid::new_v4());
        let runtime_id = format!("qemu:{}", vm_name);
        let disk_path = format!("/tmp/{}.qcow2", vm_name);

        let mut qemu_args: Vec<String> = vec![
            "-name".into(), vm_name.clone(),
            "-m".into(), spec.resources.memory_mb.unwrap_or(2048).to_string(),
            "-smp".into(), spec.resources.cpu_cores.unwrap_or(2.0).ceil().max(1.0).to_string(),
        ];

        if spec.resources.gpu {
            qemu_args.extend([
                "-vga".into(), "virtio".into(),
                "-display".into(), "gtk,gl=on".into(),
            ]);
        }

        for mount in &spec.mounts {
            qemu_args.extend([
                "-virtfs".into(),
                format!("local,path={},mount_tag={},security_model=none", mount.host_path, mount.guest_path),
            ]);
        }

        for port in &spec.ports {
            qemu_args.extend([
                "-netdev".into(),
                format!("user,id=n1,hostfwd=tcp::{}-:{}", port.host, port.guest),
                "-device".into(),
                "virtio-net-pci,netdev=n1".into(),
            ]);
        }

        qemu_args.extend([
            "-drive".into(),
            format!("file={},format=qcow2,if=virtio", disk_path),
        ]);

        for arg in &spec.command {
            qemu_args.push(arg.clone());
        }

        let args_ref: Vec<&str> = qemu_args.iter().map(|s| s.as_str()).collect();
        let output = self.run_qemu(&args_ref).await?;
        let pid = output.trim().to_string();

        let now = chrono::Utc::now();
        let job = SandboxJob {
            id: format!("qemu-{}", Uuid::new_v4()),
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

    async fn start(&self, job_id: &str) -> Result<(), String> { self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?; Ok(()) }
    async fn stop(&self, job_id: &str) -> Result<(), String> { self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?; Ok(()) }
    async fn pause(&self, job_id: &str) -> Result<(), String> { self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?; Ok(()) }
    async fn resume(&self, job_id: &str) -> Result<(), String> { self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?; Ok(()) }
    async fn wait(&self, job_id: &str) -> Result<SandboxJob, String> { self.status(job_id).await }
    async fn logs(&self, job_id: &str, _tail: Option<u32>) -> Result<Vec<String>, String> { Ok(vec![format!("qemu logs for {}", job_id)]) }
    async fn metrics(&self, job_id: &str) -> Result<JobMetrics, String> {
        self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        Ok(JobMetrics { cpu_secs: 0.0, memory_mb_peak: 0, net_rx_bytes: 0, net_tx_bytes: 0, disk_bytes_read: 0, disk_bytes_written: 0 })
    }
    async fn snapshot(&self, job_id: &str) -> Result<Snapshot, String> {
        let job = self.find_job(job_id).await.ok_or_else(|| format!("job not found: {}", job_id))?;
        Ok(Snapshot {
            id: format!("qemu-snap-{}", Uuid::new_v4()),
            job_id: job.id.clone(),
            provider: self.id().into(),
            created_at: chrono::Utc::now(),
            size_bytes: 0,
            state: "qemu-disk".into(),
        })
    }
    async fn restore(&self, _snapshot_id: &str) -> Result<SandboxJob, String> { Err("qemu restore not wired".into()) }
    async fn delete(&self, job_id: &str) -> Result<(), String> {
        let mut state = self.state.write().await;
        state.jobs.retain(|j| j.id != job_id);
        Ok(())
    }
    async fn list(&self) -> Result<Vec<SandboxJob>, String> { Ok(self.state.read().await.jobs.clone()) }
    async fn status(&self, job_id: &str) -> Result<SandboxJob, String> {
        self.find_job(job_id).await.ok_or_else(|| format!("qemu job not found: {}", job_id))
    }
}
