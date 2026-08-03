#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct UncertaintyEstimate {
    pub mean: f64,
    pub variance: f64,
    pub confidence_interval: (f64, f64),
    pub entropy: f64,
}

#[derive(Debug, Clone)]
pub struct MonteCarloUncertainty {
    pub predictions: VecDeque<f64>,
    pub dropout_rate: f64,
    pub samples: usize,
}

impl MonteCarloUncertainty {
    pub fn new(dropout_rate: f64, samples: usize) -> Self {
        Self {
            predictions: VecDeque::with_capacity(samples),
            dropout_rate,
            samples,
        }
    }

    pub fn add_prediction(&mut self, prediction: f64) {
        self.predictions.push_back(prediction);
        if self.predictions.len() > self.samples {
            self.predictions.pop_front();
        }
    }

    pub fn estimate(&self) -> UncertaintyEstimate {
        if self.predictions.is_empty() {
            return UncertaintyEstimate {
                mean: 0.0,
                variance: 0.0,
                confidence_interval: (0.0, 0.0),
                entropy: 0.0,
            };
        }

        let mean = self.predictions.iter().sum::<f64>() / self.predictions.len() as f64;
        let variance = self.predictions.iter().map(|p| (p - mean).powi(2)).sum::<f64>() / self.predictions.len() as f64;
        let std = variance.sqrt();
        let ci = (mean - 1.96 * std, mean + 1.96 * std);

        // Approximate entropy from prediction distribution
        let mut histogram = [0usize; 10];
        for p in &self.predictions {
            let bin = (*p * 10.0).floor().clamp(0.0, 9.0) as usize;
            histogram[bin] += 1;
        }
        let total = self.predictions.len() as f64;
        let entropy = histogram.iter()
            .filter(|&&c| c > 0)
            .map(|&c| {
                let p = c as f64 / total;
                -p * p.ln()
            })
            .sum();

        UncertaintyEstimate {
            mean,
            variance,
            confidence_interval: ci,
            entropy,
        }
    }

    pub fn calibration_error(&self, targets: &[f64]) -> f64 {
        if targets.len() != self.predictions.len() || targets.is_empty() {
            return 0.0;
        }
        let estimate = self.estimate();
        let mut mae = 0.0;
        for (pred, target) in self.predictions.iter().zip(targets.iter()) {
            mae += (pred - target).abs();
        }
        mae / self.predictions.len() as f64
    }
}

#[tauri::command]
pub async fn uq_estimate(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>) -> Result<UncertaintyEstimate, String> {
    let s = state.lock().await;
    let uq = s.uncertainty_quantification.lock().await;
    Ok(uq.estimate())
}

#[tauri::command]
pub async fn uq_add_prediction(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, prediction: f64) -> Result<(), String> {
    let s = state.lock().await;
    let mut uq = s.uncertainty_quantification.lock().await;
    uq.add_prediction(prediction);
    Ok(())
}
