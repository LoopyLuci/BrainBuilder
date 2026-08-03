#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[derive(Debug, Clone)]
pub struct ContinualLearning {
    pub task_history: VecDeque<TaskPerformance>,
    pub current_task_id: usize,
    pub total_tasks: usize,
}

#[derive(Debug, Clone)]
pub struct TaskPerformance {
    pub task_id: usize,
    pub accuracy: f64,
    pub loss: f64,
    pub samples_seen: usize,
}

impl ContinualLearning {
    pub fn new(total_tasks: usize) -> Self {
        Self {
            task_history: VecDeque::with_capacity(32),
            current_task_id: 0,
            total_tasks,
        }
    }

    pub fn train_on_task(&mut self, samples: usize, accuracy: f64, loss: f64) {
        let perf = TaskPerformance {
            task_id: self.current_task_id,
            accuracy,
            loss,
            samples_seen: samples,
        };
        self.task_history.push_back(perf);
        self.current_task_id += 1;
    }

    pub fn catastrophic_forgetting(&self) -> f64 {
        if self.task_history.len() < 2 {
            return 0.0;
        }
        let first = self.task_history.front().unwrap();
        let last = self.task_history.back().unwrap();
        (first.accuracy - last.accuracy).max(0.0)
    }

    pub fn progress(&self) -> f64 {
        if self.total_tasks == 0 {
            return 0.0;
        }
        (self.current_task_id as f64 / self.total_tasks as f64).min(1.0)
    }
}

#[tauri::command]
pub async fn continual_train(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, samples: usize, accuracy: f64, loss: f64) -> Result<ContinualLearning, String> {
    let s = state.lock().await;
    let mut cl = s.continual_learning.lock().await;
    cl.train_on_task(samples, accuracy, loss);
    Ok(cl.clone())
}

#[tauri::command]
pub async fn continual_progress(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>) -> Result<f64, String> {
    let s = state.lock().await;
    let cl = s.continual_learning.lock().await;
    Ok(cl.progress())
}

#[tauri::command]
pub async fn continual_forgetting(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>) -> Result<f64, String> {
    let s = state.lock().await;
    let cl = s.continual_learning.lock().await;
    Ok(cl.catastrophic_forgetting())
}
