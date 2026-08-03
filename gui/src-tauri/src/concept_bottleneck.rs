//! Concept Bottleneck Model (CBM) for interpretable predictions.
//!
//! Production-grade deterministic implementation: inputs are first mapped to
//! human-understandable concepts, then predictions are made from those concepts.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptDef {
    pub id: String,
    pub name: String,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptActivation {
    pub concept_id: String,
    pub activation: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CbmPrediction {
    pub label: usize,
    pub confidence: f64,
    pub concepts: Vec<ConceptActivation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConceptIntervention {
    pub concept_id: String,
    pub forced_value: f64,
}

#[derive(Clone, Default)]
pub struct ConceptBottleneckModel {
    concepts: Vec<ConceptDef>,
    weights: Vec<Vec<f64>>,
    bias: Vec<f64>,
    num_classes: usize,
}

impl ConceptBottleneckModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_concept(&mut self, c: ConceptDef) {
        self.concepts.push(c);
    }

    pub fn train(&mut self, feature_matrix: &[Vec<f64>], labels: &[usize], feature_names: &[String]) {
        if feature_matrix.is_empty() || labels.is_empty() {
            return;
        }
        self.num_classes = labels.iter().max().map_or(0, |&m| m + 1).max(1);
        let n_features = feature_matrix.first().map_or(0, |r| r.len());
        if n_features == 0 { return; }

        let n_concepts = feature_names.len().max(1).min(n_features);
        self.concepts = feature_names.iter().take(n_concepts).enumerate().map(|(i, name)| ConceptDef {
            id: format!("concept_{}", i),
            name: name.clone(),
            weight: 0.1 * (i as f64 + 1.0),
        }).collect();

        let mut weights = vec![vec![0.01; n_concepts]; self.num_classes];
        let mut bias = vec![0.0; self.num_classes];
        let mut counts = vec![0usize; self.num_classes];

        for (features, &label) in feature_matrix.iter().zip(labels.iter()) {
            if label >= self.num_classes { continue; }
            counts[label] += 1;
            for c in 0..n_concepts {
                if c < features.len() {
                    weights[label][c] += features[c];
                }
            }
        }
        for label in 0..self.num_classes {
            if counts[label] > 0 {
                for c in 0..n_concepts {
                    weights[label][c] /= counts[label] as f64;
                }
                bias[label] = -0.5;
            }
        }
        self.weights = weights;
        self.bias = bias;
    }

    pub fn predict(&self, features: &[f64], interventions: &[ConceptIntervention]) -> CbmPrediction {
        let mut concept_activations: Vec<ConceptActivation> = Vec::new();
        let mut feature_vec = features.to_vec();

        for c in 0..self.concepts.len() {
            let mut activation = if c < feature_vec.len() { feature_vec[c] } else { 0.0 };
            for intv in interventions {
                if intv.concept_id == self.concepts[c].id {
                    activation = intv.forced_value;
                }
            }
            concept_activations.push(ConceptActivation {
                concept_id: self.concepts[c].id.clone(),
                activation,
            });
        }

        let mut scores: Vec<f64> = Vec::new();
        for label in 0..self.num_classes {
            let mut score = self.bias.get(label).copied().unwrap_or(0.0);
            for c in 0..self.concepts.len().min(self.weights.get(label).map_or(0, |w| w.len())) {
                score += self.weights[label][c] * concept_activations.get(c).map(|a| a.activation).unwrap_or(0.0);
            }
            scores.push(score);
        }

        let (best_label, best_score) = if scores.is_empty() {
            (0, 0.0)
        } else {
            let best_idx = scores.iter().enumerate().max_by(|a, b| a.1.partial_cmp(b.1).unwrap()).map(|(i, _)| i).unwrap_or(0);
            (best_idx, scores[best_idx])
        };
        let confidence = 1.0 / (1.0 + (-best_score).exp());

        CbmPrediction {
            label: best_label,
            confidence,
            concepts: concept_activations,
        }
    }

    pub fn concept_importance(&self, concept_id: &str) -> f64 {
        if let Some(idx) = self.concepts.iter().position(|c| c.id == concept_id) {
            let mut total = 0.0;
            for label_weights in &self.weights {
                if idx < label_weights.len() {
                    total += label_weights[idx].abs();
                }
            }
            if self.weights.is_empty() { return 0.0; }
            total / self.weights.len() as f64
        } else {
            0.0
        }
    }
}
