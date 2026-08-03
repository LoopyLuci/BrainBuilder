use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActiveLearningJob {
    pub id: String,
    pub base_model: String,
    pub adapter_path: String,
    pub reward_signal: f64,
    pub status: String,
    pub created_at: String,
}

#[derive(Clone)]
pub struct ActiveLearningLoop {
    jobs: Arc<Mutex<Vec<ActiveLearningJob>>>,
    telemetry: crate::telemetry::TelemetryStore,
    base_dir: PathBuf,
}

impl ActiveLearningLoop {
    pub fn new(telemetry: crate::telemetry::TelemetryStore, base_dir: PathBuf) -> Self {
        Self { jobs: Arc::new(Mutex::new(Vec::new())), telemetry, base_dir }
    }

    pub async fn maybe_trigger(&self, category: &str) -> Result<Option<String>, String> {
        let rate = self.telemetry.success_rate(category, "*")?;
        let threshold: f64 = 0.85;
        if rate >= threshold {
            return Ok(None);
        }

        let job_id = uuid::Uuid::new_v4().to_string();
        let adapter_path = self.base_dir.join("adapters").join(format!("{}.bin", job_id));
        std::fs::create_dir_all(adapter_path.parent().unwrap()).map_err(|e| e.to_string())?;

        let job = ActiveLearningJob {
            id: job_id.clone(),
            base_model: "self".into(),
            adapter_path: adapter_path.display().to_string(),
            reward_signal: rate,
            status: "queued".into(),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        self.jobs.lock().await.push(job);
        info!(job_id=%job_id, reward=%rate, "active_learning triggered due to low success rate");
        Ok(Some(job_id))
    }

    pub async fn list_jobs(&self) -> Vec<ActiveLearningJob> {
        self.jobs.lock().await.clone()
    }
}
