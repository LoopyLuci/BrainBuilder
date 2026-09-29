#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct AdaptiveReasoning {
    pub confidence: f64,
    pub iteration_depth: usize,
    pub history: VecDeque<ReasoningStep>,
}

#[derive(Debug, Clone)]
pub struct ReasoningStep {
    pub step: usize,
    pub hypothesis: String,
    pub evidence: String,
    pub confidence: f64,
}

impl AdaptiveReasoning {
    pub fn new(initial_confidence: f64) -> Self {
        Self {
            confidence: initial_confidence,
            iteration_depth: 0,
            history: VecDeque::with_capacity(32),
        }
    }

    pub fn add_step(&mut self, hypothesis: impl Into<String>, evidence: impl Into<String>, confidence: f64) {
        let step = ReasoningStep {
            step: self.history.len(),
            hypothesis: hypothesis.into(),
            evidence: evidence.into(),
            confidence,
        };
        self.history.push_back(step);
        self.iteration_depth += 1;
        self.confidence = (self.confidence + confidence) / 2.0;
    }

    pub fn backtrack(&mut self, n: usize) -> Option<ReasoningStep> {
        for _ in 0..n.min(self.history.len()) {
            self.history.pop_back();
        }
        self.history.back().cloned()
    }

    pub fn confidence(&self) -> f64 {
        self.confidence
    }

    pub fn steps(&self) -> &VecDeque<ReasoningStep> {
        &self.history
    }
}

#[tauri::command]
pub async fn adaptive_reason(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, hypothesis: String, evidence: String, confidence: f64) -> Result<AdaptiveReasoning, String> {
    let s = state.lock().await;
    let mut r = s.adaptive_reasoning.lock().await;
    r.add_step(hypothesis, evidence, confidence);
    Ok(r.clone())
}

#[tauri::command]
pub async fn adaptive_backtrack(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, n: usize) -> Result<Option<ReasoningStep>, String> {
    let s = state.lock().await;
    let mut r = s.adaptive_reasoning.lock().await;
    Ok(r.backtrack(n))
}

#[tauri::command]
pub async fn adaptive_confidence(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>) -> Result<f64, String> {
    let s = state.lock().await;
    let r = s.adaptive_reasoning.lock().await;
    Ok(r.confidence())
}
