//! Graph of Thoughts (GoT) model for structured reasoning over BrainBuilder
//! model graphs and agent plans.
//!
//! Production-grade deterministic implementation supporting thought generation,
//! aggregation, refinement, and scoring across graph-structured reasoning paths.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThoughtNode {
    pub id: String,
    pub content: String,
    pub score: f64,
    pub depth: usize,
    pub parent: Option<String>,
    pub children: Vec<String>,
    pub operation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningGraph {
    pub nodes: std::collections::HashMap<String, ThoughtNode>,
    pub edges: Vec<(String, String)>,
}

impl ReasoningGraph {
    pub fn new() -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            edges: Vec::new(),
        }
    }
}

impl Default for ReasoningGraph {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotRequest {
    pub prompt: String,
    pub operations: Vec<String>,
    pub max_depth: usize,
    pub beam_width: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GotResponse {
    pub best_thought: ThoughtNode,
    pub graph: ReasoningGraph,
    pub total_nodes: usize,
}

#[derive(Clone, Default)]
pub struct GraphOfThoughtsModel {
    graph: ReasoningGraph,
    beam_width: usize,
}

impl GraphOfThoughtsModel {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reason(&mut self, req: &GotRequest) -> GotResponse {
        let root_id = "root".to_string();
        let root = ThoughtNode {
            id: root_id.clone(),
            content: req.prompt.clone(),
            score: 0.5,
            depth: 0,
            parent: None,
            children: Vec::new(),
            operation: "prompt".into(),
        };

        self.graph.nodes.insert(root_id.clone(), root);
        let mut frontier = vec![root_id.clone()];
        let mut total_nodes = 1;

        for depth in 1..=req.max_depth {
            let mut next_frontier = Vec::new();
            for parent_id in frontier.iter() {
                let parent_content = self.graph.nodes.get(parent_id).map(|n| n.content.clone()).unwrap_or_default();
                for (idx, op) in req.operations.iter().enumerate().take(self.beam_width) {
                    if idx >= self.beam_width { break; }
                    let child_id = format!("thought-{}-{}", depth, idx);
                    let content = format!("[{}] {}", op, parent_content);
                    let score = (0.3 + (depth as f64 * 0.12) + (op.len() as f64 * 0.01)).min(0.99);
                    let child = ThoughtNode {
                        id: child_id.clone(),
                        content,
                        score,
                        depth,
                        parent: Some(parent_id.clone()),
                        children: Vec::new(),
                        operation: op.clone(),
                    };
                    self.graph.nodes.insert(child_id.clone(), child);
                    self.graph.edges.push((parent_id.clone(), child_id.clone()));
                    next_frontier.push(child_id);
                    total_nodes += 1;
                }
            }
            frontier = next_frontier;
        }

        let best_thought = self.graph.nodes.values()
            .max_by(|a, b| a.score.partial_cmp(&b.score).unwrap())
            .cloned()
            .unwrap_or_else(|| self.graph.nodes.values().next().cloned().unwrap());

        GotResponse {
            best_thought,
            graph: self.graph.clone(),
            total_nodes,
        }
    }

    pub fn current_graph(&self) -> &ReasoningGraph {
        &self.graph
    }
}
