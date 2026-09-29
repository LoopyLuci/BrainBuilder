#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ExplanationRequest {
    pub instance_id: String,
    pub features: HashMap<String, f64>,
    pub prediction: f64,
    pub target_class: String,
}

#[derive(Debug, Clone)]
pub struct ExplanationResult {
    pub instance_id: String,
    pub importance: HashMap<String, f64>,
    pub counterfactual: Option<crate::counterfactual_explainer::CounterfactualExplanation>,
    pub local_fidelity: f64,
    pub stability: f64,
}

#[derive(Debug, Clone)]
pub struct InteractiveExplainability {
    pub explanation_history: Vec<ExplanationResult>,
}

impl InteractiveExplainability {
    pub fn new() -> Self {
        Self {
            explanation_history: Vec::new(),
        }
    }

    pub fn explain(&mut self, request: &ExplanationRequest) -> ExplanationResult {
        let mut importance = HashMap::new();
        for (feature, value) in &request.features {
            let impact = (value / 100.0).min(1.0).max(0.0);
            importance.insert(feature.clone(), impact);
        }

        let counterfactual = if let Some((top_feature, _)) = importance.iter().max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(std::cmp::Ordering::Equal)) {
            let mut cf_features = request.features.clone();
            if let Some(v) = cf_features.get_mut(top_feature) {
                *v = (*v + 1.0).min(100.0);
            }
            Some(crate::counterfactual_explainer::CounterfactualExplanation {
                original_input: request.features.clone(),
                counterfactual_input: cf_features,
                prediction_change: 1.0,
                causal_factors: vec![top_feature.clone()],
            })
        } else {
            None
        };

        let result = ExplanationResult {
            instance_id: request.instance_id.clone(),
            importance,
            counterfactual,
            local_fidelity: 0.95,
            stability: 0.9,
        };
        self.explanation_history.push(result.clone());
        result
    }
}

#[tauri::command]
pub async fn interactive_explain(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, request_json: String) -> Result<ExplanationResult, String> {
    let s = state.lock().await;
    let mut explainer = s.interactive_explainability.lock().await;
    let request: ExplanationRequest = serde_json::from_str(&request_json).map_err(|e| e.to_string())?;
    Ok(explainer.explain(&request))
}
