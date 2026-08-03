#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct PmiResult {
    pub score: f64,
    pub mutual_info: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct PmiAnalyzer {
    pub unigram_counts: HashMap<String, usize>,
    pub bigram_counts: HashMap<(String, String), usize>,
    pub total_unigrams: usize,
    pub total_bigrams: usize,
}

impl PmiAnalyzer {
    pub fn new() -> Self {
        Self {
            unigram_counts: HashMap::new(),
            bigram_counts: HashMap::new(),
            total_unigrams: 0,
            total_bigrams: 0,
        }
    }

    pub fn observe(&mut self, a: impl Into<String>, b: impl Into<String>) {
        let a = a.into();
        let b = b.into();
        *self.unigram_counts.entry(a.clone()).or_insert(0) += 1;
        *self.unigram_counts.entry(b.clone()).or_insert(0) += 1;
        *self.bigram_counts.entry((a, b)).or_insert(0) += 1;
        self.total_unigrams += 2;
        self.total_bigrams += 1;
    }

    pub fn score(&self, a: &str, b: &str) -> PmiResult {
        let unigram_total = self.total_unigrams.max(1) as f64;
        let bigram_total = self.total_bigrams.max(1) as f64;

        let p_a = *self.unigram_counts.get(a).unwrap_or(&0) as f64 / unigram_total;
        let p_b = *self.unigram_counts.get(b).unwrap_or(&0) as f64 / unigram_total;
        let p_ab = *self.bigram_counts.get(&(a.to_string(), b.to_string())).unwrap_or(&0) as f64 / bigram_total;

        let pmi = if p_ab > 0.0 && p_a > 0.0 && p_b > 0.0 {
            (p_ab / (p_a * p_b)).ln()
        } else {
            0.0
        };

        let mut mutual_info = HashMap::new();
        mutual_info.insert(format!("{}_{}", a, b), pmi);

        PmiResult {
            score: pmi,
            mutual_info,
        }
    }

    pub fn top_associations(&self, k: usize) -> Vec<((String, String), f64)> {
        let mut scores: Vec<_> = self.bigram_counts.iter()
            .map(|((a, b), _)| ((a.clone(), b.clone()), self.score(a, b).score))
            .collect();
        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);
        scores
    }
}

#[tauri::command]
pub async fn pmi_observe(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, a: String, b: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut model = s.pmi_analyzer.lock().await;
    model.observe(a, b);
    Ok(())
}

#[tauri::command]
pub async fn pmi_score(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, a: String, b: String) -> Result<PmiResult, String> {
    let s = state.lock().await;
    let model = s.pmi_analyzer.lock().await;
    Ok(model.score(&a, &b))
}
