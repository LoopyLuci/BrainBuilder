//! PowerManager/Joulara model templates and HTTP connector.
//!
//! BrainBuilder does NOT link against `joulara-*` crates directly.
//! This module exposes:
//!   - reusable model/template types derived from PowerManager patterns
//!   - a thin HTTP bridge for talking to a live Joulara/UHAD/PCE service
//!   - blueprint registration so the MetaModelBuilder can instantiate
//!     power-optimization models from these templates.

#![allow(dead_code)]

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::info;

use crate::meta_model_builder::{ModelBlueprint, ModelSpec, TrainingHints};

// ── Reusable template types ────────────────────────────────────────────

/// Simplified linear dynamics model template (from `joulara-npmc`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct LinearDynamicsTemplate {
    pub state_dim: usize,
    pub action_dim: usize,
    pub a_coefficient: f32,
    pub b_coefficient: f32,
}

impl LinearDynamicsTemplate {
    pub fn new(state_dim: usize, action_dim: usize) -> Self {
        Self {
            state_dim,
            action_dim,
            a_coefficient: 0.995,
            b_coefficient: 0.02,
        }
    }

    /// Produce a BrainBuilder `ModelSpec` from this template.
    pub fn to_model_spec(&self, task: &str, training_data_path: impl Into<String>, output_path: impl Into<String>) -> ModelSpec {
        let mut parameters = HashMap::new();
        parameters.insert("state_dim".into(), serde_json::Value::Number(self.state_dim.into()));
        parameters.insert("action_dim".into(), serde_json::Value::Number(self.action_dim.into()));
        parameters.insert("a_coefficient".into(), serde_json::Value::Number(serde_json::Number::from_f64(self.a_coefficient as f64).unwrap_or_else(|| serde_json::Number::from(0))));
        parameters.insert("b_coefficient".into(), serde_json::Value::Number(serde_json::Number::from_f64(self.b_coefficient as f64).unwrap_or_else(|| serde_json::Number::from(0))));

        let mut metrics = HashMap::new();
        metrics.insert("status".into(), serde_json::Value::String("planned".into()));
        metrics.insert("source".into(), serde_json::Value::String("powermanager-linear-dynamics".into()));

        ModelSpec {
            blueprint_id: "powermanager-linear-dynamics".into(),
            id: format!("pm-linear-{}-{}", task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        }
    }
}

/// MPC controller template (from `joulara-npmc::mpc`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct MpcControllerTemplate {
    pub fast_horizon: usize,
    pub w_power: f64,
    pub w_temp: f64,
    pub w_effort: f64,
}

impl Default for MpcControllerTemplate {
    fn default() -> Self {
        Self {
            fast_horizon: 4,
            w_power: 1.0,
            w_temp: 2.0,
            w_effort: 0.1,
        }
    }
}

impl MpcControllerTemplate {
    pub fn to_blueprint(&self) -> ModelBlueprint {
        let hints = TrainingHints {
            optimizer: "ipm-qp".into(),
            loss: "box-constrained-qp".into(),
            default_epochs: 1,
            batch_size: self.fast_horizon,
            features: vec![
                "power_limit_w".into(),
                "temperature_c".into(),
                "domain_id".into(),
            ],
            constraints: vec![
                format!("lower=5.0"),
                format!("upper=100.0"),
                "horizon=".to_string() + &self.fast_horizon.to_string(),
            ],
        };

        ModelBlueprint {
            id: "powermanager-mpc".into(),
            task: "power-optimization".into(),
            model_type: "mpc".into(),
            name: "PowerManager MPC".into(),
            category: "systems-control".into(),
            description: "Primal-dual IPM QP solver for per-domain power limits".into(),
            training_hints: hints,
        }
    }
}

/// Workload classifier template (from `joulara-npmc::workload`).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct WorkloadClassifierTemplate {
    pub classes: Vec<String>,
}

impl Default for WorkloadClassifierTemplate {
    fn default() -> Self {
        Self {
            classes: vec![
                "idle".into(),
                "productivity".into(),
                "gaming".into(),
                "sustained".into(),
            ],
        }
    }
}

impl WorkloadClassifierTemplate {
    pub fn to_blueprint(&self) -> ModelBlueprint {
        ModelBlueprint {
            id: "powermanager-workload".into(),
            task: "workload-classification".into(),
            model_type: "classifier".into(),
            name: "Workload Classifier".into(),
            category: "systems-control".into(),
            description: "Classify active workload for adaptive MPC weight tuning".into(),
            training_hints: TrainingHints {
                optimizer: "rule-based".into(),
                loss: "none".into(),
                default_epochs: 0,
                batch_size: 1,
                features: vec!["cpu_util".into(), "gpu_util".into(), "thermal_c".into()],
                constraints: vec!["offline-capable".into()],
            },
        }
    }
}

// ── Blueprint registry ────────────────────────────────────────────────

#[derive(Clone, Default, Debug)]
pub struct PowerManagerModelRegistry {
    blueprints: Arc<Mutex<HashMap<String, ModelBlueprint>>>,
    templates: Arc<Mutex<HashMap<String, serde_json::Value>>>,
}

impl PowerManagerModelRegistry {
    pub fn new() -> Self {
        let mut bps = HashMap::new();
        let mut tmpl = HashMap::new();

        let mpc_bp = MpcControllerTemplate::default().to_blueprint();
        let wl_bp = WorkloadClassifierTemplate::default().to_blueprint();
        let linear_bp = ModelBlueprint {
            id: "powermanager-linear-dynamics".into(),
            task: "dynamics-linearization".into(),
            model_type: "linear".into(),
            name: "Linear Dynamics".into(),
            category: "systems-control".into(),
            description: "First-order linear fallback for power state transitions".into(),
            training_hints: TrainingHints {
                optimizer: "none".into(),
                loss: "none".into(),
                default_epochs: 0,
                batch_size: 1,
                features: vec!["power_w".into(), "temp_c".into()],
                constraints: vec!["deterministic".into()],
            },
        };

        bps.insert(mpc_bp.id.clone(), mpc_bp);
        bps.insert(wl_bp.id.clone(), wl_bp);
        bps.insert(linear_bp.id.clone(), linear_bp);

        tmpl.insert(
            "linear-dynamics".into(),
            serde_json::to_value(LinearDynamicsTemplate::new(9, 3)).unwrap(),
        );
        tmpl.insert(
            "mpc-controller".into(),
            serde_json::to_value(MpcControllerTemplate::default()).unwrap(),
        );

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

    pub async fn build_mpc_spec(
        &self,
        task: &str,
        training_data_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Result<ModelSpec, String> {
        let bps = self.blueprints.lock().await;
        let bp = bps.get("powermanager-mpc").ok_or_else(|| String::from("missing powermanager-mpc blueprint"))?;

        let mut parameters = HashMap::new();
        parameters.insert("task".into(), serde_json::Value::String(task.into()));
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

        info!(task=%task, "power manager MPC spec generated");

        Ok(ModelSpec {
            blueprint_id: bp.id.clone(),
            id: format!("pm-mpc-{}-{}", task, uuid::Uuid::new_v4()),
            parameters,
            training_data_path: training_data_path.into(),
            output_path: output_path.into(),
            metrics,
        })
    }
}

// ── Live service bridge ────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct PowerManagerBridge {
    base_url: Arc<Mutex<String>>,
    http: reqwest::Client,
    registry: PowerManagerModelRegistry,
}

impl PowerManagerBridge {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            base_url: Arc::new(Mutex::new(base_url.into())),
            http: reqwest::Client::new(),
            registry: PowerManagerModelRegistry::new(),
        }
    }

    pub async fn set_base_url(&self, url: impl Into<String>) {
        *self.base_url.lock().await = url.into();
    }

    pub async fn health(&self) -> Result<PmHealth, String> {
        let url = format!("{}/health", self.base_url.lock().await);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("PowerManager health check failed: {}", status));
        }
        let body = resp.json::<PmHealth>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn list_domains(&self) -> Result<Vec<PmDomain>, String> {
        let url = format!("{}/api/v1/domains", self.base_url.lock().await);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("list_domains failed: {}", status));
        }
        let body = resp.json::<Vec<PmDomain>>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn apply_power_limit(&self, domain_id: &str, watts: f64) -> Result<PmIntentResult, String> {
        let url = format!("{}/api/v1/intents", self.base_url.lock().await);
        let payload = serde_json::json!({
            "domain_id": domain_id,
            "action": { "PowerLimit": { "sustained_w": watts, "burst_w": watts * 1.2 } }
        });
        let resp = self.http.post(&url).json(&payload).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("apply_power_limit failed: {}", status));
        }
        let body = resp.json::<PmIntentResult>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn recent_telemetry(&self, limit: usize) -> Result<Vec<PmTelemetrySample>, String> {
        let url = format!("{}/api/v1/telemetry?limit={}", self.base_url.lock().await, limit);
        let resp = self.http.get(&url).send().await.map_err(|e| e.to_string())?;
        let status = resp.status();
        if !status.is_success() {
            return Err(format!("recent_telemetry failed: {}", status));
        }
        let body = resp.json::<Vec<PmTelemetrySample>>().await.map_err(|e| e.to_string())?;
        Ok(body)
    }

    pub async fn list_blueprints(&self) -> Vec<ModelBlueprint> {
        self.registry.list_blueprints().await
    }

    pub async fn build_mpc_spec(
        &self,
        task: &str,
        training_data_path: impl Into<String>,
        output_path: impl Into<String>,
    ) -> Result<ModelSpec, String> {
        self.registry.build_mpc_spec(task, training_data_path, output_path).await
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PmHealth {
    pub service: String,
    pub status: String,
    pub domains: usize,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PmDomain {
    pub id: String,
    pub name: String,
    pub parent_id: Option<String>,
    pub children_ids: Vec<String>,
    pub descriptor: serde_json::Value,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PmIntentResult {
    pub domain_id: String,
    pub accepted: bool,
    pub applied_w: Option<f64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PmTelemetrySample {
    pub domain_id: String,
    pub timestamp: String,
    pub power_w: f64,
    pub temp_c: f64,
    pub cpu_util: Option<f64>,
    pub gpu_util: Option<f64>,
}

// ── Tauri commands ───────────────────────────────────────────────────

#[tauri::command]
pub async fn pm_health(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
) -> Result<PmHealth, String> {
    let s = state.lock().await;
    s.power_manager.health().await
}

#[tauri::command]
pub async fn pm_list_domains(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
) -> Result<Vec<PmDomain>, String> {
    let s = state.lock().await;
    s.power_manager.list_domains().await
}

#[tauri::command]
pub async fn pm_apply_power_limit(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    domain_id: String,
    watts: f64,
) -> Result<PmIntentResult, String> {
    let s = state.lock().await;
    s.power_manager.apply_power_limit(&domain_id, watts).await
}

#[tauri::command]
pub async fn pm_recent_telemetry(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    limit: usize,
) -> Result<Vec<PmTelemetrySample>, String> {
    let s = state.lock().await;
    s.power_manager.recent_telemetry(limit).await
}

#[tauri::command]
pub async fn pm_list_blueprints(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
) -> Result<Vec<crate::meta_model_builder::ModelBlueprint>, String> {
    let s = state.lock().await;
    Ok(s.power_manager.list_blueprints().await)
}

#[tauri::command]
pub async fn pm_build_mpc_spec(
    state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>,
    task: String,
    training_data_path: String,
    output_path: String,
) -> Result<crate::meta_model_builder::ModelSpec, String> {
    let s = state.lock().await;
    s.power_manager.build_mpc_spec(&task, training_data_path, output_path).await
}
