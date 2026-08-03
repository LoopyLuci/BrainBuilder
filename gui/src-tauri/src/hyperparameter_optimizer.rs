#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct Parameter {
    pub name: String,
    pub value: f64,
    pub lower_bound: f64,
    pub upper_bound: f64,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub iteration: usize,
    pub parameters: Vec<Parameter>,
    pub score: f64,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone)]
pub struct HyperparameterSearch {
    pub strategy: SearchStrategy,
    pub iterations: usize,
    pub budget_ms: u64,
    pub results: VecDeque<SearchResult>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchStrategy {
    Grid,
    Random,
    Bayesian,
}

impl HyperparameterSearch {
    pub fn new(strategy: SearchStrategy, budget_ms: u64) -> Self {
        Self {
            strategy,
            iterations: 0,
            budget_ms,
            results: VecDeque::with_capacity(128),
        }
    }

    pub fn step(&mut self, params: &[Parameter], score: f64, elapsed_ms: u64) -> SearchResult {
        let result = SearchResult {
            iteration: self.iterations,
            parameters: params.to_vec(),
            score,
            elapsed_ms,
        };
        self.results.push_back(result.clone());
        self.iterations += 1;
        result
    }

    pub fn best(&self) -> Option<&SearchResult> {
        self.results.iter().max_by(|a, b| a.score.partial_cmp(&b.score).unwrap_or(std::cmp::Ordering::Equal))
    }

    pub fn progress(&self) -> f64 {
        let total_ms: u64 = self.results.iter().map(|r| r.elapsed_ms).sum();
        (total_ms as f64 / self.budget_ms as f64).min(1.0)
    }
}

#[tauri::command]
pub async fn hpo_step(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, params_json: String, score: f64, elapsed_ms: u64) -> Result<SearchResult, String> {
    let s = state.lock().await;
    let mut hpo = s.hyperparameter_search.lock().await;
    let params: Vec<Parameter> = serde_json::from_str(&params_json).map_err(|e| e.to_string())?;
    Ok(hpo.step(&params, score, elapsed_ms))
}

#[tauri::command]
pub async fn hpo_best(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>) -> Result<Option<SearchResult>, String> {
    let s = state.lock().await;
    let hpo = s.hyperparameter_search.lock().await;
    Ok(hpo.best().cloned())
}
