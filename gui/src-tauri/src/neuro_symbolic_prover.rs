#![allow(dead_code)]
use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::{HashMap, VecDeque};

#[derive(Debug, Clone)]
pub struct Clause {
    pub id: usize,
    pub head: String,
    pub body: Vec<String>,
    pub embedding: Vec<f64>,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct ProofTree {
    pub root: String,
    pub nodes: Vec<ProofNode>,
}

#[derive(Debug, Clone)]
pub struct ProofNode {
    pub clause_id: usize,
    pub children: Vec<ProofNode>,
    pub depth: usize,
}

#[derive(Debug, Clone)]
pub struct NeuroSymbolicProver {
    pub clauses: Vec<Clause>,
    pub facts: HashMap<String, bool>,
    pub embedding_dim: usize,
    pub max_depth: usize,
}

impl NeuroSymbolicProver {
    pub fn new(embedding_dim: usize, max_depth: usize) -> Self {
        Self {
            clauses: Vec::new(),
            facts: HashMap::new(),
            embedding_dim,
            max_depth,
        }
    }

    pub fn add_fact(&mut self, fact: impl Into<String>, truth: bool) {
        self.facts.insert(fact.into(), truth);
    }

    pub fn add_clause(&mut self, head: impl Into<String>, body: Vec<String>, weight: f64) {
        let id = self.clauses.len();
        let head_string = head.into();
        let body_strings: Vec<String> = body.iter().map(|s| s.to_string()).collect();
        let embedding = self.embed_clause(&head_string, &body_strings);
        self.clauses.push(Clause {
            id,
            head: head_string,
            body,
            embedding,
            weight,
        });
    }

    pub fn prove(&self, goal: &str, max_steps: usize) -> Option<ProofTree> {
        if self.facts.get(goal) == Some(&true) {
            return Some(ProofTree {
                root: goal.to_string(),
                nodes: vec![ProofNode { clause_id: 0, children: Vec::new(), depth: 0 }],
            });
        }

        let mut visited = std::collections::HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back((goal.to_string(), 0, Vec::new()));

        while let Some((current_goal, depth, ancestors)) = queue.pop_front() {
            if depth >= self.max_depth || depth >= max_steps { continue; }
            if visited.contains(&current_goal) { continue; }
            if !visited.insert(current_goal.clone()) { continue; }

            for clause in &self.clauses {
                if clause.head == current_goal {
                    let mut child_nodes = Vec::new();
                    let mut all_children_proven = true;
                    for body_literal in &clause.body {
                        if self.facts.get(body_literal) == Some(&true) {
                            child_nodes.push(ProofNode { clause_id: clause.id, children: Vec::new(), depth: depth + 1 });
                        } else {
                            all_children_proven = false;
                            let mut new_ancestors = ancestors.clone();
                            new_ancestors.push(current_goal.clone());
                            queue.push_back((body_literal.clone(), depth + 1, new_ancestors));
                        }
                    }
                    if all_children_proven {
                        return Some(ProofTree {
                            root: goal.to_string(),
                            nodes: vec![ProofNode { clause_id: clause.id, children: child_nodes, depth }],
                        });
                    }
                }
            }
        }
        None
    }

    pub fn clause_count(&self) -> usize {
        self.clauses.len()
    }

    pub fn fact_count(&self) -> usize {
        self.facts.len()
    }

    fn embed_clause(&self, head: &str, body: &[String]) -> Vec<f64> {
        let mut embedding = vec![0.0; self.embedding_dim];
        let text = format!("{} {}", head, body.join(" "));
        let words: Vec<&str> = text.split_whitespace().collect();
        for (i, word) in words.iter().enumerate() {
            let idx = (word.len() + i) % self.embedding_dim;
            embedding[idx] += 1.0;
        }
        let norm: f64 = embedding.iter().map(|x| x * x).sum::<f64>().sqrt();
        if norm > 0.0 {
            for val in embedding.iter_mut() {
                *val /= norm;
            }
        }
        embedding
    }
}

#[tauri::command]
pub async fn neurosymbolic_prove(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, goal: String, max_steps: usize) -> Result<Option<ProofTree>, String> {
    let s = state.lock().await;
    let model = s.neuro_symbolic_prover.lock().await;
    Ok(model.prove(&goal, max_steps))
}

#[tauri::command]
pub async fn neurosymbolic_add_clause(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, head: String, body: Vec<String>, weight: f64) -> Result<(), String> {
    let s = state.lock().await;
    let mut model = s.neuro_symbolic_prover.lock().await;
    model.add_clause(head, body, weight);
    Ok(())
}
