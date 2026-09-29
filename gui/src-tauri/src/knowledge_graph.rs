#![allow(dead_code)]
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct KnowledgeGraph {
    pub entities: HashMap<String, Entity>,
    pub relations: Vec<Relation>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Entity {
    pub id: String,
    pub entity_type: String,
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Relation {
    pub source: String,
    pub target: String,
    pub relation_type: String,
    pub weight: f64,
}

#[derive(Debug, Clone)]
pub struct Subgraph {
    pub nodes: Vec<String>,
    pub edges: Vec<Relation>,
}

impl KnowledgeGraph {
    pub fn new() -> Self {
        Self {
            entities: HashMap::new(),
            relations: Vec::new(),
        }
    }

    pub fn add_entity(&mut self, entity: Entity) {
        self.entities.insert(entity.id.clone(), entity);
    }

    pub fn add_relation(&mut self, source: impl Into<String>, target: impl Into<String>, relation_type: impl Into<String>, weight: f64) {
        self.relations.push(Relation {
            source: source.into(),
            target: target.into(),
            relation_type: relation_type.into(),
            weight,
        });
    }

    pub fn neighbors(&self, node_id: &str) -> Vec<Relation> {
        self.relations.iter()
            .filter(|r| r.source == node_id || r.target == node_id)
            .cloned()
            .collect()
    }

    pub fn subgraph(&self, center: &str, depth: usize) -> Subgraph {
        let mut nodes = std::collections::HashSet::new();
        let mut edges = Vec::new();
        let mut queue = VecDeque::new();
        queue.push_back((center.to_string(), 0));
        nodes.insert(center.to_string());

        while let Some((current, d)) = queue.pop_front() {
            if d >= depth { continue; }
            for rel in &self.relations {
                let next = if rel.source == current {
                    Some((rel.target.clone(), rel.clone()))
                } else if rel.target == current {
                    Some((rel.source.clone(), rel.clone()))
                } else {
                    None
                };
                if let Some((neighbor, rel)) = next {
                    if nodes.insert(neighbor.clone()) {
                        queue.push_back((neighbor.clone(), d + 1));
                    }
                    edges.push(rel);
                }
            }
        }

        Subgraph {
            nodes: nodes.into_iter().collect(),
            edges,
        }
    }

    pub fn shortest_path(&self, source: &str, target: &str) -> Option<Vec<String>> {
        if source == target { return Some(vec![source.to_string()]); }
        let mut visited = std::collections::HashSet::new();
        let mut parent: HashMap<String, String> = HashMap::new();
        let mut queue = VecDeque::new();
        queue.push_back(source.to_string());
        visited.insert(source.to_string());

        while let Some(current) = queue.pop_front() {
            for rel in &self.relations {
                let neighbor = if rel.source == current {
                    Some(&rel.target)
                } else if rel.target == current {
                    Some(&rel.source)
                } else {
                    None
                };
                if let Some(n) = neighbor {
                    if !visited.contains(n.as_str()) {
                        visited.insert(n.clone());
                        parent.insert(n.clone(), current.clone());
                        if n == target {
                            let mut path = vec![target.to_string()];
                            let mut curr = target.to_string();
                            while let Some(p) = parent.get(&curr) {
                                path.push(p.clone());
                                curr = p.clone();
                            }
                            path.reverse();
                            return Some(path);
                        }
                        queue.push_back(n.clone());
                    }
                }
            }
        }
        None
    }
}

use std::sync::Arc;
use tokio::sync::Mutex;
use std::collections::VecDeque;

#[tauri::command]
pub async fn kg_add_entity(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, entity_json: String) -> Result<(), String> {
    let s = state.lock().await;
    let mut kg = s.knowledge_graph.lock().await;
    let entity: Entity = serde_json::from_str(&entity_json).map_err(|e| e.to_string())?;
    kg.add_entity(entity);
    Ok(())
}

#[tauri::command]
pub async fn kg_add_relation(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, source: String, target: String, relation_type: String, weight: f64) -> Result<(), String> {
    let s = state.lock().await;
    let mut kg = s.knowledge_graph.lock().await;
    kg.add_relation(source, target, relation_type, weight);
    Ok(())
}

#[tauri::command]
pub async fn kg_neighbors(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, node_id: String) -> Result<Vec<Relation>, String> {
    let s = state.lock().await;
    let kg = s.knowledge_graph.lock().await;
    Ok(kg.neighbors(&node_id))
}

#[tauri::command]
pub async fn kg_shortest_path(state: tauri::State<'_, Arc<Mutex<crate::self_improving_commands::AppStateExt>>>, source: String, target: String) -> Result<Option<Vec<String>>, String> {
    let s = state.lock().await;
    let kg = s.knowledge_graph.lock().await;
    Ok(kg.shortest_path(&source, &target))
}
