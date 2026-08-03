#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct TemporalPointProcess {
    pub events: VecDeque<TemporalEvent>,
    pub rate: f64,
    pub window_size: usize,
}

#[derive(Debug, Clone)]
pub struct TemporalEvent {
    pub timestamp: f64,
    pub intensity: f64,
    pub context: HashMap<String, f64>,
}

#[derive(Debug, Clone)]
pub struct ForecastResult {
    pub next_event_time: f64,
    pub confidence: f64,
    pub expected_intensity: f64,
}

impl TemporalPointProcess {
    pub fn new(initial_rate: f64, window_size: usize) -> Self {
        Self {
            events: VecDeque::with_capacity(window_size),
            rate: initial_rate,
            window_size,
        }
    }

    pub fn record(&mut self, timestamp: f64, intensity: f64, context: HashMap<String, f64>) {
        self.events.push_back(TemporalEvent { timestamp, intensity, context });
        if self.events.len() > self.window_size {
            self.events.pop_front();
        }
        self.rate = self.events.iter().map(|e| e.intensity).sum::<f64>() / self.events.len().max(1) as f64;
    }

    pub fn forecast(&self, horizon: f64) -> ForecastResult {
        if self.events.is_empty() {
            return ForecastResult {
                next_event_time: horizon,
                confidence: 0.0,
                expected_intensity: 0.0,
            };
        }
        let last_time = self.events.back().unwrap().timestamp;
        let next_event_time = last_time + (1.0 / self.rate.max(1e-6));
        let confidence = (self.events.len() as f64 / self.window_size as f64).min(1.0);
        ForecastResult {
            next_event_time,
            confidence,
            expected_intensity: self.rate,
        }
    }

    pub fn intensity_at(&self, t: f64) -> f64 {
        if self.events.is_empty() {
            return 0.0;
        }
        let mut intensity = self.rate;
        for event in &self.events {
            let dt = t - event.timestamp;
            if dt > 0.0 {
                intensity += event.intensity * (-dt).exp();
            }
        }
        intensity.max(0.0)
    }
}

#[tauri::command]
pub async fn tpp_forecast(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, horizon: f64) -> Result<ForecastResult, String> {
    let s = state.lock().await;
    let model = s.temporal_point_process.lock().await;
    Ok(model.forecast(horizon))
}

#[tauri::command]
pub async fn tpp_record(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, timestamp: f64, intensity: f64, context_json: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut model = s.temporal_point_process.lock().await;
    let context: HashMap<String, f64> = serde_json::from_str(&context_json).map_err(|e| e.to_string())?;
    model.record(timestamp, intensity, context);
    Ok(())
}
