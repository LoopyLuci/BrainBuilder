use super::super::tool_executor::executor::Tool;
use async_trait::async_trait;
use serde_json::Value;
use std::sync::Arc;
use crate::tool_executor::world::Workspace;
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct MemoryEntry {
    pub id: String,
    pub text: String,
    pub tags: Vec<String>,
    pub created_at: String,
}

impl MemoryEntry {
    pub fn embedding(&self) -> Vec<f32> {
        simple_embedding(&self.text)
    }
}

pub fn simple_embedding(text: &str) -> Vec<f32> {
    let mut embedding = vec![0.0f32; 64];
    let tokens: Vec<&str> = text.split_whitespace().collect();
    for (idx, token) in tokens.iter().take(64).enumerate() {
        let sum = token.bytes().map(|b| b as f32).sum::<f32>();
        embedding[idx] = (sum % 100.0) / 100.0;
    }
    embedding
}

pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let mut dot = 0.0;
    let mut norm_a = 0.0;
    let mut norm_b = 0.0;
    for i in 0..a.len().min(b.len()) {
        dot += a[i] * b[i];
        norm_a += a[i] * a[i];
        norm_b += b[i] * b[i];
    }
    if norm_a == 0.0 || norm_b == 0.0 {
        0.0
    } else {
        dot / (norm_a.sqrt() * norm_b.sqrt())
    }
}

pub struct VectorMemoryStore {
    workspace: Arc<Workspace>,
    path: String,
}

impl VectorMemoryStore {
    pub fn new(workspace: Arc<Workspace>, path: &str) -> Self {
        let store = Self { workspace: workspace.clone(), path: path.to_string() };
        store.init();
        store
    }

    fn init(&self) {
        if let Ok(conn) = self.conn() {
            let _ = conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS memories (\
                  id TEXT PRIMARY KEY, \
                  text TEXT NOT NULL, \
                  tags TEXT, \
                  created_at TEXT\
                ); \
                CREATE TABLE IF NOT EXISTS memory_embeddings (\
                  id TEXT PRIMARY KEY, \
                  embedding BLOB\
                );",
            );
        }
    }

    fn conn(&self) -> Result<Connection, String> {
        let path = self.workspace.resolve(&self.path).map_err(|e| e.to_string())?;
        Connection::open(path).map_err(|e| e.to_string())
    }

    pub async fn insert(&self, entry: MemoryEntry) -> Result<(), String> {
        let id = entry.id.clone();
        let text = entry.text.clone();
        let tags = serde_json::to_string(&entry.tags).map_err(|e| e.to_string())?;
        let created_at = entry.created_at.clone();
        let embedding = entry.embedding();
        let embedding_blob: Vec<u8> = embedding.iter().flat_map(|v| v.to_le_bytes()).collect();
        self.workspace.create_file(&self.path, "").map_err(|e| e.to_string())?;
        let conn = self.conn()?;
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, text, tags, created_at) VALUES (?1, ?2, ?3, ?4)",
            &[&id, &text, &tags, &created_at],
        ).map_err(|e| e.to_string())?;
        conn.execute(
            "INSERT OR REPLACE INTO memory_embeddings (id, embedding) VALUES (?1, ?2)",
            rusqlite::params![&id, &embedding_blob],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn query(&self, text: &str, limit: usize) -> Vec<(MemoryEntry, f32)> {
        let query_emb = simple_embedding(text);
        let conn = match self.conn() {
            Ok(c) => c,
            Err(_) => return Vec::new(),
        };

        let mut rows: Vec<(String, f32)> = Vec::new();
        let mut stmt = match conn.prepare("SELECT id, embedding FROM memory_embeddings") {
            Ok(s) => s,
            Err(_) => return Vec::new(),
        };
        let iter = match stmt.query_map([], |row| {
            let id: String = row.get(0).unwrap_or_default();
            let blob: Vec<u8> = row.get(1).unwrap_or_default();
            let mut emb = Vec::new();
            for chunk in blob.chunks(4) {
                if chunk.len() == 4 {
                    let mut bytes = [0u8; 4];
                    bytes.copy_from_slice(chunk);
                    emb.push(f32::from_le_bytes(bytes));
                }
            }
            Ok((id, emb))
        }) {
            Ok(i) => i,
            Err(_) => return Vec::new(),
        };

        for r in iter {
            if let Ok((id, emb)) = r {
                let sim = cosine_similarity(&query_emb, &emb);
                rows.push((id, sim));
            }
        }

        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        rows.truncate(limit);

        let mut results = Vec::new();
        for (id, score) in rows {
            let mut entry = MemoryEntry {
                id,
                text: String::new(),
                tags: Vec::new(),
                created_at: String::new(),
            };
            if let Ok(mut stmt2) = conn.prepare("SELECT text, tags, created_at FROM memories WHERE id = ?1") {
                if let Ok(mut rows2) = stmt2.query(&[&entry.id]) {
                    if let Ok(Some(row)) = rows2.next() {
                        entry.text = row.get(0).unwrap_or_default();
                        entry.tags = serde_json::from_str(&row.get::<_, String>(1).unwrap_or_default()).unwrap_or_default();
                        entry.created_at = row.get(2).unwrap_or_default();
                    }
                }
            }
            results.push((entry, score));
        }
        results
    }
}

pub struct VectorMemoryTool;
#[async_trait]
impl Tool for VectorMemoryTool {
    fn name(&self) -> &'static str { "memory.vector_query" }
    async fn execute(&self, args: &Value) -> Result<Value, String> {
        let query = args.get("query").and_then(|v| v.as_str()).ok_or("query required")?;
        let limit = args.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;
        let workspace = Arc::new(Workspace::new("./workspace"));
        let store = VectorMemoryStore::new(workspace, "memory/vectors.sqlite");
        let matches = store.query(query, limit).await;
        let results: Vec<Value> = matches.into_iter().map(|(entry, score)| {
            serde_json::json!({
                "id": entry.id,
                "text": entry.text,
                "tags": entry.tags,
                "score": score,
                "created_at": entry.created_at,
            })
        }).collect();
        Ok(serde_json::json!({"query": query, "matches": results}))
    }
}
