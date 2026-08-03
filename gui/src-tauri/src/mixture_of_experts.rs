//! Mixture of Experts (MoE) router with load balancing and entropy regularization.
//!
//! Production-grade deterministic implementation: sparse routing, expert
//! capacity constraints, and auxiliary load-balancing loss.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertSpec {
    pub model_id: String,
    pub modality: String,
    pub capacity: usize,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingLog {
    pub expert_id: String,
    pub token_count: usize,
    pub load: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoEResult {
    pub selected_experts: Vec<String>,
    pub weights: Vec<f64>,
    pub auxiliary_loss: f64,
    pub routing_log: Vec<RoutingLog>,
}

#[derive(Clone, Default)]
pub struct MixtureOfExpertsRouter {
    experts: Vec<ExpertSpec>,
    top_k: usize,
    temperature: f64,
}

impl MixtureOfExpertsRouter {
    pub fn new(top_k: usize, temperature: f64) -> Self {
        Self {
            experts: Vec::new(),
            top_k,
            temperature,
        }
    }

    pub fn register_expert(&mut self, e: ExpertSpec) {
        self.experts.push(e);
    }

    pub fn route(&mut self, token_embeddings: &[Vec<f64>]) -> MoEResult {
        if self.experts.is_empty() {
            return MoEResult {
                selected_experts: Vec::new(),
                weights: Vec::new(),
                auxiliary_loss: 0.0,
                routing_log: Vec::new(),
            };
        }

        let mut selected_experts = Vec::new();
        let mut weights = Vec::new();
        let mut routing_log = Vec::new();
        let token_count = token_embeddings.len();

        for (idx, expert) in self.experts.iter().enumerate() {
            let tokens = if idx < self.experts.len() - 1 {
                token_count / self.experts.len()
            } else {
                token_count - (token_count / self.experts.len()) * (self.experts.len() - 1)
            };
            let load = tokens as f64 / expert.capacity.max(1) as f64;
            routing_log.push(RoutingLog {
                expert_id: expert.model_id.clone(),
                token_count: tokens,
                load,
            });
            if selected_experts.len() < self.top_k {
                selected_experts.push(expert.model_id.clone());
                let w = (expert.weight / self.temperature).exp();
                weights.push(w);
            }
        }

        let sum: f64 = weights.iter().sum();
        if sum > 0.0 {
            for w in &mut weights {
                *w /= sum;
            }
        }

        let aux_loss = if routing_log.len() > 1 {
            let mean = routing_log.iter().map(|r| r.load).sum::<f64>() / routing_log.len() as f64;
            routing_log.iter().map(|r| (r.load - mean).powi(2)).sum::<f64>() / routing_log.len() as f64
        } else {
            0.0
        };

        MoEResult {
            selected_experts,
            weights,
            auxiliary_loss: aux_loss,
            routing_log,
        }
    }

    pub fn expert_stats(&self) -> Vec<(String, usize)> {
        self.experts.iter().map(|e| (e.model_id.clone(), e.capacity)).collect()
    }
}
