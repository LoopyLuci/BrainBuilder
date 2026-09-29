use std::sync::Arc;
use tokio::sync::Mutex;
use serde_json::json;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MetaSuggestion {
    pub id: String,
    pub param: String,
    pub current_value: serde_json::Value,
    pub proposed_value: serde_json::Value,
    pub reason: String,
    pub confidence: f64,
    pub applied: bool,
}

#[derive(Clone)]
pub struct MetaController {
    telemetry: crate::telemetry::TelemetryStore,
    suggestions: Arc<Mutex<Vec<MetaSuggestion>>>,
}

impl MetaController {
    pub fn new(telemetry: crate::telemetry::TelemetryStore) -> Self {
        Self { telemetry, suggestions: Arc::new(Mutex::new(Vec::new())) }
    }

    pub async fn suggest(&self) -> Result<Vec<MetaSuggestion>, String> {
        let mut out = Vec::new();

        // Example heuristic: if tool invocation success rate < 0.9, suggest lowering temperature
        let rate = self.telemetry.success_rate("tauri_command", "*")?;
        if rate < 0.9 {
            out.push(MetaSuggestion {
                id: uuid::Uuid::new_v4().to_string(),
                param: "llm.temperature".into(),
                current_value: json!(0.7),
                proposed_value: json!(0.5),
                reason: format!("Low command success rate {:.2}", rate),
                confidence: 0.8,
                applied: false,
            });
        }

        // Example: if training latency > 30s median, suggest smaller rank
        let train_events = self.telemetry.query("training", 20)?;
        if !train_events.is_empty() {
            let avg_latency: f64 = train_events.iter().map(|e| e.latency_ms as f64).sum::<f64>() / train_events.len() as f64;
            if avg_latency > 30_000.0 {
                out.push(MetaSuggestion {
                    id: uuid::Uuid::new_v4().to_string(),
                    param: "training.default_rank".into(),
                    current_value: json!(16),
                    proposed_value: json!(8),
                    reason: format!("High training latency {:.0}ms", avg_latency),
                    confidence: 0.7,
                    applied: false,
                });
            }
        }

        self.suggestions.lock().await.extend(out.clone());
        info!(suggestions=out.len(), "meta suggestions generated");
        Ok(out)
    }

    pub async fn apply(&self, id: &str) -> Result<(), String> {
        let mut list = self.suggestions.lock().await;
        if let Some(s) = list.iter_mut().find(|s| s.id == id) {
            s.applied = true;
            info!(suggestion_id=id, param=%s.param, "meta suggestion applied");
            Ok(())
        } else {
            Err("Suggestion not found".into())
        }
    }

    pub async fn list(&self) -> Vec<MetaSuggestion> {
        self.suggestions.lock().await.clone()
    }
}
