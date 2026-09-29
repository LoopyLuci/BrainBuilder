use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
use serde_json::json;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AuditEntry {
    pub id: String,
    pub ts: String,
    pub action: String,
    pub actor: String,
    pub reason: String,
    pub snapshot: String,
}

#[derive(Clone)]
pub struct SafetyHarness {
    audit: Arc<Mutex<Vec<AuditEntry>>>,
    checkpoints: Arc<Mutex<Vec<String>>>,
    base_dir: PathBuf,
}

impl SafetyHarness {
    pub fn new(base_dir: PathBuf) -> Self {
        std::fs::create_dir_all(&base_dir).ok();
        Self {
            audit: Arc::new(Mutex::new(Vec::new())),
            checkpoints: Arc::new(Mutex::new(Vec::new())),
            base_dir,
        }
    }

    pub async fn record(&self, action: &str, actor: &str, reason: &str, snapshot: &str) {
        let entry = AuditEntry {
            id: uuid::Uuid::new_v4().to_string(),
            ts: chrono::Utc::now().to_rfc3339(),
            action: action.into(),
            actor: actor.into(),
            reason: reason.into(),
            snapshot: snapshot.into(),
        };
        self.audit.lock().await.push(entry.clone());
        info!(audit_id=%entry.id, action=%action, actor=%actor, "audit recorded");
    }

    pub async fn checkpoint(&self, label: &str) -> Result<String, String> {
        let id = uuid::Uuid::new_v4().to_string();
        let path = self.base_dir.join("checkpoints").join(format!("{}.json", id));
        std::fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let snapshot = json!({ "label": label, "ts": chrono::Utc::now().to_rfc3339() });
        std::fs::write(&path, snapshot.to_string()).map_err(|e| e.to_string())?;
        self.checkpoints.lock().await.push(id.clone());
        self.record("checkpoint", "system", label, &snapshot.to_string()).await;
        info!(checkpoint_id=%id, label=%label, "checkpoint created");
        Ok(id)
    }

    pub async fn rollback(&self, checkpoint_id: &str) -> Result<(), String> {
        let list = self.checkpoints.lock().await.clone();
        if !list.iter().any(|id| id == checkpoint_id) {
            return Err("Checkpoint not found".into());
        }
        self.record("rollback", "system", checkpoint_id, checkpoint_id).await;
        warn!(checkpoint_id=%checkpoint_id, "rollback executed");
        Ok(())
    }

    pub async fn audit_log(&self) -> Vec<AuditEntry> {
        self.audit.lock().await.clone()
    }
}
