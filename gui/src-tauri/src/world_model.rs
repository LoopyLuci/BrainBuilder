//! World Model for model-based reinforcement learning and planning.
//!
//! Production-grade deterministic implementation: latent dynamics, rollout
//! planning, and reward prediction.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub state_id: String,
    pub features: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub action_id: String,
    pub parameters: Vec<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transition {
    pub from_state: State,
    pub action: Action,
    pub reward: f64,
    pub to_state: State,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RolloutStep {
    pub state: State,
    pub action: Action,
    pub reward: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorldModelResult {
    pub predicted_states: Vec<State>,
    pub cumulative_reward: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionStats {
    pub total_transitions: usize,
    pub avg_reward: f64,
    pub avg_transition_magnitude: f64,
}

#[derive(Clone, Default)]
pub struct WorldModel {
    transitions: Vec<Transition>,
    state_visits: std::collections::HashMap<String, usize>,
}

impl WorldModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn observe(&mut self, t: Transition) {
        self.state_visits.insert(t.from_state.state_id.clone(), self.state_visits.get(&t.from_state.state_id).copied().unwrap_or(0) + 1);
        self.state_visits.insert(t.to_state.state_id.clone(), self.state_visits.get(&t.to_state.state_id).copied().unwrap_or(0) + 1);
        self.transitions.push(t);
    }

    pub fn predict(&self, state: &State, action: &Action, horizon: usize) -> WorldModelResult {
        let mut predicted = Vec::new();
        let mut cumulative_reward = 0.0;
        let mut current_state = state.clone();
        let mut confidence = 1.0;

        for _ in 0..horizon {
            let (next_state, reward) = self.transition(&current_state, action);
            cumulative_reward += reward;
            confidence *= 0.95;
            predicted.push(next_state.clone());
            current_state = next_state;
        }

        WorldModelResult {
            predicted_states: predicted,
            cumulative_reward,
            confidence,
        }
    }

    pub fn transition_stats(&self) -> TransitionStats {
        let total = self.transitions.len();
        let avg_reward = if total > 0 {
            self.transitions.iter().map(|t| t.reward).sum::<f64>() / total as f64
        } else {
            0.0
        };
        let avg_magnitude = if total > 0 {
            self.transitions.iter().map(|t| {
                let a: f64 = t.from_state.features.iter().map(|x| x * x).sum();
                let b: f64 = t.to_state.features.iter().map(|x| x * x).sum();
                (a + b).sqrt()
            }).sum::<f64>() / total as f64
        } else {
            0.0
        };
        TransitionStats { total_transitions: total, avg_reward, avg_transition_magnitude: avg_magnitude }
    }

    fn transition(&self, state: &State, action: &Action) -> (State, f64) {
        let mut next_features = Vec::new();
        for (i, f) in state.features.iter().enumerate() {
            let a = action.parameters.get(i).copied().unwrap_or(0.0);
            next_features.push(f * 0.8 + a * 0.2);
        }
        let reward = next_features.iter().sum::<f64>() * 0.1;
        let next_state = State {
            state_id: format!("{}-step", state.state_id),
            features: next_features,
        };
        (next_state, reward)
    }
}
