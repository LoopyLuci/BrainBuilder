#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CounterfactualExplanation {
    pub original_input: HashMap<String, f64>,
    pub counterfactual_input: HashMap<String, f64>,
    pub prediction_change: f64,
    pub causal_factors: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CounterfactualEngine {
    pub feature_constraints: HashMap<String, (f64, f64)>,
}

impl CounterfactualEngine {
    pub fn new() -> Self {
        Self {
            feature_constraints: HashMap::new(),
        }
    }

    pub fn set_constraint(&mut self, feature: impl Into<String>, min: f64, max: f64) {
        self.feature_constraints.insert(feature.into(), (min, max));
    }

    pub fn explain(&self, original: &HashMap<String, f64>, prediction: f64, target: f64) -> Option<CounterfactualExplanation> {
        let mut counterfactual = original.clone();
        let delta = target - prediction;
        let mut causal_factors = Vec::new();

        for (feature, value) in original.iter() {
            if let Some(&(min, max)) = self.feature_constraints.get(feature) {
                let perturbation = delta * 0.1;
                let mut new_value = value + perturbation;
                new_value = new_value.clamp(min, max);
                if (new_value - value).abs() > 1e-6 {
                    causal_factors.push(feature.clone());
                }
                counterfactual.insert(feature.clone(), new_value);
            }
        }

        let prediction_change = target - prediction;
        if causal_factors.is_empty() {
            None
        } else {
            Some(CounterfactualExplanation {
                original_input: original.clone(),
                counterfactual_input: counterfactual,
                prediction_change,
                causal_factors,
            })
        }
    }
}

#[tauri::command]
pub async fn counterfactual_explain(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, input_json: String, prediction: f64, target: f64) -> Result<Option<CounterfactualExplanation>, String> {
    let s = state.lock().await;
    let engine = s.counterfactual_engine.lock().await;
    let original: HashMap<String, f64> = serde_json::from_str(&input_json).map_err(|e| e.to_string())?;
    Ok(engine.explain(&original, prediction, target))
}
