#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::HashMap;


#[derive(Debug, Clone, serde::Deserialize)]
pub struct ClientUpdate {
    pub client_id: String,
    pub weights: Vec<f64>,
    pub samples: usize,
    pub loss: f64,
}

#[derive(Debug, Clone)]
pub struct AggregatedModel {
    pub weights: Vec<f64>,
    pub round: usize,
    pub participating_clients: usize,
}

#[derive(Debug, Clone)]
pub struct FederatedLearning {
    pub global_weights: Vec<f64>,
    pub current_round: usize,
    pub clients: HashMap<String, ClientProfile>,
}

#[derive(Debug, Clone)]
pub struct ClientProfile {
    pub client_id: String,
    pub samples_seen: usize,
    pub last_round: usize,
    pub reliability: f64,
}

impl FederatedLearning {
    pub fn new(initial_weights: Vec<f64>) -> Self {
        Self {
            global_weights: initial_weights,
            current_round: 0,
            clients: HashMap::new(),
        }
    }

    pub fn register_client(&mut self, client_id: impl Into<String>) {
        let id = client_id.into();
        self.clients.insert(id.clone(), ClientProfile {
            client_id: id,
            samples_seen: 0,
            last_round: 0,
            reliability: 1.0,
        });
    }

    pub fn aggregate(&mut self, updates: &[ClientUpdate]) -> AggregatedModel {
        if updates.is_empty() {
            return AggregatedModel {
                weights: self.global_weights.clone(),
                round: self.current_round,
                participating_clients: 0,
            };
        }

        let dim = self.global_weights.len();
        let mut new_weights = vec![0.0; dim];
        let mut total_samples = 0;

        for update in updates {
            for (i, w) in update.weights.iter().enumerate().take(dim) {
                new_weights[i] += w * update.samples as f64;
            }
            total_samples += update.samples;
            self.clients.entry(update.client_id.clone())
                .and_modify(|c| { c.samples_seen += update.samples; c.last_round = self.current_round; });
        }

        if total_samples > 0 {
            for w in new_weights.iter_mut() {
                *w /= total_samples as f64;
            }
        }

        self.global_weights = new_weights.clone();
        self.current_round += 1;

        AggregatedModel {
            weights: new_weights,
            round: self.current_round,
            participating_clients: updates.len(),
        }
    }

    pub fn round(&self) -> usize {
        self.current_round
    }

    pub fn client_count(&self) -> usize {
        self.clients.len()
    }
}

#[tauri::command]
pub async fn federated_aggregate(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, updates_json: String) -> Result<AggregatedModel, String> {
    let s = state.lock().await;
    let mut fl = s.federated_learning.lock().await;
    let updates: Vec<ClientUpdate> = serde_json::from_str(&updates_json).map_err(|e| e.to_string())?;
    Ok(fl.aggregate(&updates))
}

#[tauri::command]
pub async fn federated_register(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, client_id: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut fl = s.federated_learning.lock().await;
    fl.register_client(client_id);
    Ok(())
}
