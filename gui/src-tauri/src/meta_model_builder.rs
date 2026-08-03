//  Custom meta model-builder for automated AI/ML model generation.
// 
//  This module provides a registry of custom model traits/blueprints and a
//  builder that assembles them into concrete model specs for training or
//  inference. It is offline-capable and does not require external AI services.

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
pub struct FieldSpec {
    pub name: String,
    pub dtype: String,
    pub required: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TrainingHints {
    pub optimizer: String,
    pub loss: String,
    pub default_epochs: usize,
    pub batch_size: usize,
    pub features: Vec<String>,
    pub constraints: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelSpec {
    pub blueprint_id: String,
    pub id: String,
    pub parameters: HashMap<String, serde_json::Value>,
    pub training_data_path: String,
    pub output_path: String,
    pub metrics: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ModelBlueprint {
    pub id: String,
    pub task: String,
    pub model_type: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub training_hints: TrainingHints,
}

#[derive(Clone, Default)]
pub struct MetaModelBuilder {
    blueprints: Arc<Mutex<HashMap<String, ModelBlueprint>>>,
}

impl MetaModelBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn register_blueprint(&self, bp: ModelBlueprint) {
        self.blueprints.lock().await.insert(bp.id.clone(), bp);
    }

    pub async fn list_blueprints(&self) -> Vec<ModelBlueprint> {
        self.blueprints.lock().await.values().cloned().collect()
    }

    pub async fn build_model_spec(
        &self,
        blueprint_id: &str,
        task: &str,
        training_data_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Result<ModelSpec, String> {
        let bps = self.blueprints.lock().await;
        let bp = bps.get(blueprint_id).ok_or_else(|| format!("unknown blueprint: {}", blueprint_id))?;

        let mut parameters = HashMap::new();
        parameters.insert("task".into(), serde_json::Value::String(task.into()));
        parameters.insert("blueprint_name".into(), serde_json::Value::String(bp.name.clone()));
        parameters.insert("optimizer".into(), serde_json::Value::String(bp.training_hints.optimizer.clone()));
        parameters.insert("loss".into(), serde_json::Value::String(bp.training_hints.loss.clone()));
        parameters.insert("epochs".into(), serde_json::Value::Number(bp.training_hints.default_epochs.into()));
        parameters.insert("batch_size".into(), serde_json::Value::Number(bp.training_hints.batch_size.into()));
        parameters.insert("features".into(), serde_json::Value::Array(
            bp.training_hints.features.iter().map(|f| serde_json::Value::String(f.clone())).collect()
        ));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("blueprint_category".into(), serde_json::Value::String(bp.category.clone()));

        info!(
            blueprint=%blueprint_id,
            task=%task,
            "meta model builder spec generated"
        );

        Ok(ModelSpec {
            blueprint_id: blueprint_id.into(),
            id: format!("{}-{}", blueprint_id, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        })
    }

    pub async fn recommend_blueprint(&self, task: &str) -> Option<ModelBlueprint> {
        let task_lower = task.to_lowercase();
        let bps = self.blueprints.lock().await;
        let mut best: Option<(usize, &ModelBlueprint)> = None;

        for bp in bps.values() {
            let score = bp.category.to_lowercase().matches(&task_lower).count()
                + bp.description.to_lowercase().matches(&task_lower).count()
                + bp.training_hints.features.iter().filter(|f| task_lower.contains(f.to_lowercase().as_str())).count();
            if let Some((best_score, _)) = best {
                if score > best_score {
                    best = Some((score, bp));
                }
            } else if score > 0 {
                best = Some((score, bp));
            }
        }

        best.map(|(_, bp)| bp.clone())
    }
}
