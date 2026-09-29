//! CompressionAgent model templates and HTTP connector.
//!
//! BrainBuilder does NOT embed CompressionAgent directly.
//! This module exposes:
//!   - reusable compression model/template types derived from CompressionAgent
//!   - a thin HTTP bridge for talking to a live CompressionAgent service
//!   - blueprint registration so the MetaModelBuilder can instantiate
//!     compression models from these templates.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::meta_model_builder::{ModelBlueprint, ModelSpec, TrainingHints};

// ── Reusable template types ────────────────────────────────────────────

/// Atomic compressor template (token-level whitespace normalization).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AtomicCompressorTemplate {
    pub name: String,
    pub version: String,
}

impl AtomicCompressorTemplate {
    pub fn new() -> Self {
        Self {
            name: "atomic".into(),
            version: "0.1.0".into(),
        }
    }

    pub fn to_model_spec(&self, task: &str, training_data_path: impl Into<String>, output_path: impl Into<String>) -> ModelSpec {
        let mut parameters = HashMap::new();
        parameters.insert("strategy".into(), serde_json::Value::String("atomic".into()));
        parameters.insert("version".into(), serde_json::Value::String(self.version.clone()));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("source".into(), serde_json::Value::String("compression-agent-atomic".into()));

        ModelSpec {
            blueprint_id: "compression-agent-atomic".into(),
            id: format!("ca-atomic-{}-{}", task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        }
    }
}

/// Semantic compressor template (filler-word removal + normalization).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SemanticCompressorTemplate {
    pub name: String,
    pub version: String,
    pub filler_patterns: Vec<String>,
}

impl SemanticCompressorTemplate {
    pub fn new() -> Self {
        Self {
            name: "semantic".into(),
            version: "0.1.0".into(),
            filler_patterns: vec![
                "um ".into(),
                "uh ".into(),
                "you know".into(),
                "like ".into(),
                "so ".into(),
            ],
        }
    }

    pub fn to_model_spec(&self, task: &str, training_data_path: impl Into<String>, output_path: impl Into<String>) -> ModelSpec {
        let mut parameters = HashMap::new();
        parameters.insert("strategy".into(), serde_json::Value::String("semantic".into()));
        parameters.insert("version".into(), serde_json::Value::String(self.version.clone()));
        parameters.insert("filler_patterns".into(), serde_json::Value::Array(
            self.filler_patterns.iter().map(|p| serde_json::Value::String(p.clone())).collect()
        ));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("source".into(), serde_json::Value::String("compression-agent-semantic".into()));

        ModelSpec {
            blueprint_id: "compression-agent-semantic".into(),
            id: format!("ca-semantic-{}-{}", task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        }
    }
}

/// Adaptive compressor template (length-based strategy selection).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AdaptiveCompressorTemplate {
    pub name: String,
    pub version: String,
    pub threshold: usize,
}

impl AdaptiveCompressorTemplate {
    pub fn new() -> Self {
        Self {
            name: "adaptive".into(),
            version: "0.1.0".into(),
            threshold: 1000,
        }
    }

    pub fn with_threshold(mut self, threshold: usize) -> Self {
        self.threshold = threshold;
        self
    }

    pub fn to_model_spec(&self, task: &str, training_data_path: impl Into<String>, output_path: impl Into<String>) -> ModelSpec {
        let mut parameters = HashMap::new();
        parameters.insert("strategy".into(), serde_json::Value::String("adaptive".into()));
        parameters.insert("version".into(), serde_json::Value::String(self.version.clone()));
        parameters.insert("threshold".into(), serde_json::Value::Number(self.threshold.into()));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("source".into(), serde_json::Value::String("compression-agent-adaptive".into()));

        ModelSpec {
            blueprint_id: "compression-agent-adaptive".into(),
            id: format!("ca-adaptive-{}-{}", task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        }
    }
}

// ── Blueprint registry ────────────────────────────────────────────────

#[derive(Clone, Default, Debug)]
pub struct CompressionAgentModelRegistry {
    blueprints: Arc<Mutex<HashMap<String, ModelBlueprint>>>,
    templates: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl CompressionAgentModelRegistry {
    pub fn new() -> Self {
        let mut bps = HashMap::new();
        let mut tmpl = HashMap::new();

        let atomic_bp = ModelBlueprint {
            id: "compression-agent-atomic".into(),
            task: "text-compression".into(),
            model_type: "compressor".into(),
            name: "Atomic Compressor".into(),
            category: "compression".into(),
            description: "Token-level whitespace normalization".into(),
            training_hints: TrainingHints {
                optimizer: "none".into(),
                loss: "none".into(),
                default_epochs: 0,
                batch_size: 1,
                features: vec!["input_text".into()],
                constraints: vec!["deterministic".into(), "stateless".into()],
            },
        };

        let semantic_bp = ModelBlueprint {
            id: "compression-agent-semantic".into(),
            task: "text-compression".into(),
            model_type: "compressor".into(),
            name: "Semantic Compressor".into(),
            category: "compression".into(),
            description: "Meaning-preserving compression with filler removal".into(),
            training_hints: TrainingHints {
                optimizer: "none".into(),
                loss: "none".into(),
                default_epochs: 0,
                batch_size: 1,
                features: vec!["input_text".into(), "filler_patterns".into()],
                constraints: vec!["deterministic".into(), "lossless-semantics".into()],
            },
        };

        let adaptive_bp = ModelBlueprint {
            id: "compression-agent-adaptive".into(),
            task: "text-compression".into(),
            model_type: "compressor".into(),
            name: "Adaptive Compressor".into(),
            category: "compression".into(),
            description: "Length-aware strategy selection between atomic and semantic".into(),
            training_hints: TrainingHints {
                optimizer: "none".into(),
                loss: "none".into(),
                default_epochs: 0,
                batch_size: 1,
                features: vec!["input_text".into(), "threshold".into()],
                constraints: vec!["deterministic".into(), "length-threshold".into()],
            },
        };

        bps.insert(atomic_bp.id.clone(), atomic_bp);
        bps.insert(semantic_bp.id.clone(), semantic_bp);
        bps.insert(adaptive_bp.id.clone(), adaptive_bp);

        tmpl.insert("atomic".into(), serde_json::to_value(AtomicCompressorTemplate::new()).unwrap());
        tmpl.insert("semantic".into(), serde_json::to_value(SemanticCompressorTemplate::new()).unwrap());
        tmpl.insert("adaptive".into(), serde_json::to_value(AdaptiveCompressorTemplate::new()).unwrap());

        Self {
            blueprints: Arc::new(Mutex::new(bps)),
            templates: Arc::new(Mutex::new(tmpl)),
        }
    }

    pub async fn register_blueprint(&self, bp: ModelBlueprint) {
        self.blueprints.lock().await.insert(bp.id.clone(), bp);
    }

    pub async fn list_blueprints(&self) -> Vec<ModelBlueprint> {
        self.blueprints.lock().await.values().cloned().collect()
    }

    pub async fn get_template(&self, name: &str) -> Option<serde_json::Value> {
        self.templates.lock().await.get(name).cloned()
    }

    pub async fn build_compressor_spec(
        &self,
        strategy: &str,
        task: &str,
        training_data_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Result<ModelSpec, String> {
        let bps = self.blueprints.lock().await;
        let bp_id = match strategy {
            "atomic" => "compression-agent-atomic",
            "semantic" => "compression-agent-semantic",
            "adaptive" => "compression-agent-adaptive",
            other => return Err(format!("unknown compression strategy: {}", other)),
        };
        let bp = bps.get(bp_id).ok_or_else(|| format!("missing blueprint: {}", bp_id))?;

        let mut parameters = HashMap::new();
        parameters.insert("task".into(), serde_json::Value::String(task.into()));
        parameters.insert("strategy".into(), serde_json::Value::String(strategy.into()));
        parameters.insert("blueprint_name".into(), serde_json::Value::String(bp.name.clone()));
        parameters.insert("optimizer".into(), serde_json::Value::String(bp.training_hints.optimizer.clone()));
        parameters.insert("loss".into(), serde_json::Value::String(bp.training_hints.loss.clone()));
        parameters.insert("epochs".into(), serde_json::Value::Number(bp.training_hints.default_epochs.into()));
        parameters.insert("batch_size".into(), serde_json::Value::Number(bp.training_hints.default_epochs.into()));
        parameters.insert("features".into(), serde_json::Value::Array(
            bp.training_hints.features.iter().map(|f| serde_json::Value::String(f.clone())).collect()
        ));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("blueprint_category".into(), serde_json::Value::String(bp.category.clone()));

        info!(strategy=%strategy, task=%task, "compression agent spec generated");

        Ok(ModelSpec {
            blueprint_id: bp.id.clone(),
            id: format!("ca-{}-{}-{}", strategy, task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        })
    }
}

// ── Live service bridge ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct CompressionAgentBridge {
    base_url: Arc<Mutex<String>>,
    http: reqwest::Client,
    registry: CompressionAgentModelRegistry,
}

impl CompressionAgentBridge {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: Arc::new(Mutex::new(base_url.into())),
            http: reqwest::Client::new(),
            registry: CompressionAgentModelRegistry::new(),
        }
    }

    pub async fn set_base_url(&self, url: impl Into<String>) {
        *self.base_url.lock().await = url.into();
    }

    pub async fn health(&self) -> Result<CaHealth, String> {
        let url = format!("{}/health", self.base_url.lock().await);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("CompressionAgent health check failed: {}", status));
        }
        let body = resp.json::<CaHealth>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn compress(&self, content: impl Into<String>, strategy: impl Into<String>) -> Result<CaCompressionResult, String> {
        let url = format!("{}/api/v1/compress", self.base_url.lock().await);
        let payload = serde_json::json!({
            "content": content.into(),
            "strategy": strategy.into(),
        });
        let resp = self.http.post(&url).json(&payload).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("compress failed: {}", status));
        }
        let body = resp.json::<CaCompressionResult>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn recent_stats(&self, limit: usize) -> Result<Vec<CaCompressionStats>, String> {
        let url = format!("{}/api/v1/telemetry/stats?limit={}", self.base_url.lock().await, limit);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("recent_stats failed: {}", status));
        }
        let body = resp.json::<Vec<CaCompressionStats>>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn list_blueprints(&self) -> Vec<ModelBlueprint> {
        self.registry.list_blueprints().await
    }

    pub async fn build_compressor_spec(
        &self,
        strategy: &str,
        task: &str,
        training_data_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Result<ModelSpec, String> {
        self.registry.build_compressor_spec(strategy, task, training_data_path, output_path).await
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaHealth {
    pub service: String,
    pub status: String,
    pub version: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaCompressionResult {
    pub content: String,
    pub metadata: CaCompressionMetadata,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaCompressionMetadata {
    pub original_length: usize,
    pub compressed_length: usize,
    pub ratio: f64,
    pub tokens_removed: u32,
    pub strategy: String,
    pub model_version: String,
    pub checksum: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CaCompressionStats {
    pub input_length: usize,
    pub output_length: usize,
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub compression_ratio: f64,
    pub strategy: String,
    pub timestamp: String,
}

// ── Tauri commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn ca_health(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
) -> Result<CaHealth, String> {
    let s = state.lock().await;
    s.compression_agent.health().await
}

#[tauri::command]
pub async fn ca_compress(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    content: String,
    strategy: String,
) -> Result<CaCompressionResult, String> {
    let s = state.lock().await;
    s.compression_agent.compress(content, strategy).await
}

#[tauri::command]
pub async fn ca_recent_stats(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    limit: usize,
) -> Result<Vec<CaCompressionStats>, String> {
    let s = state.lock().await;
    s.compression_agent.recent_stats(limit).await
}

#[tauri::command]
pub async fn ca_list_blueprints(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
) -> Result<Vec<crate::meta_model_builder::ModelBlueprint>, String> {
    let s = state.lock().await;
    Ok(s.compression_agent.list_blueprints().await)
}

#[tauri::command]
pub async fn ca_build_compressor_spec(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    strategy: String,
    task: String,
    training_data_path: String,
    output_path: String,
) -> Result<crate::meta_model_builder::ModelSpec, String> {
    let s = state.lock().await;
    s.compression_agent.build_compressor_spec(&strategy, &task, training_data_path, output_path).await
}
