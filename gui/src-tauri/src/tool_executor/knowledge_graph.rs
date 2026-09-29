use super::super::tool_executor::executor::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use crate::tool_executor::world::Workspace;
use chrono::Utc;

#[derive(Debug, Clone)]
pub struct KnowledgeNode {
    pub id: String,
    pub label: String,
    pub entity_type: String,
    pub properties: Vec<(String, String)>,
    pub created_at: String,
}

#[derive(Debug, Clone)]
pub struct KnowledgeEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub relation: String,
    pub weight: f64,
    pub created_at: String,
}

pub struct KnowledgeGraphStore {
    workspace: Arc<Workspace>,
    path: &'static str,
}

impl KnowledgeGraphStore {
    pub fn new(workspace: Arc<Workspace>) -> Self {
        let store = Self { workspace: workspace.clone(), path: "knowledge/graph.sqlite" };
        let _ = store.init();
        store
    }

    fn init(&self) -> Result<(), String> {
        self.workspace.create_file(self.path, "")?;
        let path = self.workspace.resolve(self.path).map_err(|e| e.to_string())?;
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS nodes (\
              id TEXT PRIMARY KEY, \
              label TEXT NOT NULL, \
              entity_type TEXT NOT NULL, \
              properties TEXT, \
              created_at TEXT NOT NULL\
            ); \
            CREATE TABLE IF NOT EXISTS edges (\
              id TEXT PRIMARY KEY, \
              source TEXT NOT NULL, \
              target TEXT NOT NULL, \
              relation TEXT NOT NULL, \
              weight REAL NOT NULL, \
              created_at TEXT NOT NULL\
            ); \
            CREATE INDEX IF NOT EXISTS idx_edges_source ON edges(source); \
            CREATE INDEX IF NOT EXISTS idx_edges_target ON edges(target);",
        ).map_err(|e| e.to_string())
    }

    pub async fn add_node(&self, node: KnowledgeNode) -> Result<(), String> {
        let properties = serde_json::to_string(&node.properties).map_err(|e| e.to_string())?;
        let path = self.workspace.resolve(self.path).map_err(|e| e.to_string())?;
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO nodes (id, label, entity_type, properties, created_at) VALUES (?1, ?2, ?3, ?4, ?5)",
            rusqlite::params![&node.id, &node.label, &node.entity_type, &properties, &node.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn add_edge(&self, edge: KnowledgeEdge) -> Result<(), String> {
        let path = self.workspace.resolve(self.path).map_err(|e| e.to_string())?;
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO edges (id, source, target, relation, weight, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            rusqlite::params![&edge.id, &edge.source, &edge.target, &edge.relation, &edge.weight, &edge.created_at],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn query_relations(&self, entity: &str) -> Result<Vec<(KnowledgeEdge, bool)>, String> {
        let path = self.workspace.resolve(self.path).map_err(|e| e.to_string())?;
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        let mut stmt = conn.prepare("SELECT id, source, target, relation, weight, created_at FROM edges WHERE source = ?1 OR target = ?1").map_err(|e| e.to_string())?;
        let iter = stmt.query_map(&[&entity], |row| {
            Ok((
                KnowledgeEdge {
                    id: row.get(0)?,
                    source: row.get(1)?,
                    target: row.get(2)?,
                    relation: row.get(3)?,
                    weight: row.get(4)?,
                    created_at: row.get(5)?,
                },
                row.get::<_, String>(1)? == entity,
            ))
        }).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for r in iter {
            match r { Ok(item) => out.push(item), Err(e) => return Err(e.to_string()) };
        }
        Ok(out)
    }

    pub async fn extract_entities(&self, text: &str) -> Result<Vec<KnowledgeNode>, String> {
        let words: Vec<&str> = text.split_whitespace().collect();
        let mut entities = Vec::new();
        for (idx, word) in words.iter().enumerate() {
            if word.len() > 2 && word.chars().any(|c| c.is_uppercase()) {
                let id = format!("entity-{}-{}", Utc::now().timestamp_millis(), idx);
                entities.push(KnowledgeNode {
                    id,
                    label: word.to_string(),
                    entity_type: "extracted".into(),
                    properties: vec![("source".into(), "heuristic".into())],
                    created_at: Utc::now().to_rfc3339(),
                });
            }
        }
        for node in &entities {
            self.add_node(node.clone()).await?;
        }
        Ok(entities)
    }
}

pub struct KnowledgeGraphTool;
#[async_trait]
impl Tool for KnowledgeGraphTool {
    fn name(&self) -> &'static str { "kg.query" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let entity = args.get("entity").and_then(|v| v.as_str()).ok_or("entity required")?;
        let workspace = Arc::new(Workspace::new("./workspace"));
        let store = KnowledgeGraphStore::new(workspace);
        let relations = store.query_relations(entity).await?;
        let out: Vec<Value> = relations.into_iter().map(|(edge, is_source)| {
            serde_json::json!({
                "id": edge.id,
                "source": edge.source,
                "target": edge.target,
                "relation": edge.relation,
                "weight": edge.weight,
                "direction": if is_source { "outgoing" } else { "incoming" },
            })
        }).collect();
        Ok(serde_json::json!({"entity": entity, "relations": out}))
    }
}

pub struct EntityExtractionTool;
#[async_trait]
impl Tool for EntityExtractionTool {
    fn name(&self) -> &'static str { "kg.extract_entities" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let text = args.get("text").and_then(|v| v.as_str()).ok_or("text required")?;
        let workspace = Arc::new(Workspace::new("./workspace"));
        let store = KnowledgeGraphStore::new(workspace);
        let entities = store.extract_entities(text).await?;
        let out: Vec<Value> = entities.into_iter().map(|node| {
            serde_json::json!({
                "id": node.id,
                "label": node.label,
                "entity_type": node.entity_type,
                "properties": node.properties,
                "created_at": node.created_at,
            })
        }).collect();
        Ok(serde_json::json!({"text": text, "entities": out}))
    }
}
