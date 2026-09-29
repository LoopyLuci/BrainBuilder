#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct FlowResult {
    pub flow_type: FlowType,
    pub analysis: String,
    pub bottlenecks: Vec<String>,
    pub utilization: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlowType {
    DataFlow,
    ControlFlow,
    TokenFlow,
    StateFlow,
}

#[derive(Debug, Clone)]
pub struct FlowAnalyzer {
    pub flow_type: FlowType,
    pub nodes: Vec<FlowNode>,
    pub edges: Vec<FlowEdge>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowNode {
    pub id: String,
    pub capacity: f64,
    pub utilization: f64,
    pub latency_ms: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FlowEdge {
    pub source: String,
    pub target: String,
    pub throughput: f64,
    pub congestion: f64,
}

impl FlowAnalyzer {
    pub fn new(flow_type: FlowType) -> Self {
        Self {
            flow_type,
            nodes: Vec::new(),
            edges: Vec::new(),
        }
    }

    pub fn add_node(&mut self, node: FlowNode) {
        self.nodes.push(node);
    }

    pub fn add_edge(&mut self, source: impl Into<String>, target: impl Into<String>, throughput: f64) {
        self.edges.push(FlowEdge {
            source: source.into(),
            target: target.into(),
            throughput,
            congestion: 0.0,
        });
    }

    pub fn bottlenecks(&self, threshold: f64) -> Vec<String> {
        self.edges.iter()
            .filter(|e| e.congestion > threshold)
            .map(|e| format!("{} -> {} ({:.2})", e.source, e.target, e.congestion))
            .collect()
    }

    pub fn simulate(&mut self, load: f64) -> FlowResult {
        for edge in self.edges.iter_mut() {
            edge.congestion = (edge.throughput * load).min(1.0);
        }

        let utilization = if self.nodes.is_empty() { 0.0 } else {
            self.nodes.iter().map(|n| n.utilization).sum::<f64>() / self.nodes.len() as f64
        };

        let bottleneck_list = self.bottlenecks(0.7);
        let analysis = if bottleneck_list.is_empty() {
            format!("Flow OK at load {:.0}%", load * 100.0)
        } else {
            format!("Congestion detected at load {:.0}%", load * 100.0)
        };

        FlowResult {
            flow_type: self.flow_type,
            analysis,
            bottlenecks: bottleneck_list,
            utilization,
        }
    }
}

#[tauri::command]
pub async fn flow_simulate(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, load: f64) -> Result<FlowResult, String> {
    let s = state.lock().await;
    let mut analyzer = s.flow_analyzer.lock().await;
    Ok(analyzer.simulate(load))
}

#[tauri::command]
pub async fn flow_add_node(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, node_json: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut analyzer = s.flow_analyzer.lock().await;
    let node: FlowNode = serde_json::from_str(&node_json).map_err(|e| e.to_string())?;
    analyzer.add_node(node);
    Ok(())
}
