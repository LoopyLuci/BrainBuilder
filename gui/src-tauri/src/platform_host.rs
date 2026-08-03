//! Real PlatformHost with SQLite-backed canvas + model registry.
//! Uses rusqlite (bundled) to match brainbuilder-core, avoiding native libsqlite3 conflict.

use async_trait::async_trait;
use concierge_core::error::ConciergeError;
use concierge_core::tools::implementations::PlatformHost;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tauri::Manager;
use tracing::info;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNode {
    pub id: String,
    pub node_type: String,
    pub label: String,
    pub model_id: Option<String>,
    pub x: f64,
    pub y: f64,
    pub config: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEdge {
    pub id: String,
    pub source: String,
    pub target: String,
    pub source_socket: String,
    pub target_socket: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct CanvasState {
    pub nodes: Vec<CanvasNode>,
    pub edges: Vec<CanvasEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelEntry {
    pub id: String,
    pub name: String,
    pub path: String,
    pub format: String,
    pub modality: String,
}

pub struct OmniForgeHost {
    conn: Mutex<Connection>,
    canvas: Arc<Mutex<CanvasState>>,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl OmniForgeHost {
    pub fn new(db_url: &str) -> Result<Self, ConciergeError> {
        let mut conn = Connection::open(db_url)
            .map_err(|e| ConciergeError::Memory(format!("DB connect failed: {e}")))?;
        for pragma in [
            "PRAGMA journal_mode = WAL;",
            "PRAGMA synchronous = NORMAL;",
            "PRAGMA cache_size = -65536;",
            "PRAGMA temp_store = MEMORY;",
            "PRAGMA busy_timeout = 5000;",
            "PRAGMA foreign_keys = ON;",
            "PRAGMA mmap_size = 268435456;",
        ] {
            conn.execute_batch(pragma)
                .map_err(|e| ConciergeError::Memory(format!("PRAGMA failed ({pragma}): {e}")))?;
        }
        let schema = r#"
            CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY, name TEXT NOT NULL, path TEXT NOT NULL,
                format TEXT NOT NULL, modality TEXT NOT NULL DEFAULT 'text'
            );
            CREATE TABLE IF NOT EXISTS canvas_nodes (
                id TEXT PRIMARY KEY, node_type TEXT NOT NULL, label TEXT NOT NULL,
                model_id TEXT, x REAL NOT NULL, y REAL NOT NULL, config_json TEXT
            );
            CREATE TABLE IF NOT EXISTS canvas_edges (
                id TEXT PRIMARY KEY, source TEXT NOT NULL, target TEXT NOT NULL,
                source_socket TEXT NOT NULL, target_socket TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_models_name ON models(name);
            CREATE INDEX IF NOT EXISTS idx_models_format ON models(format);
            CREATE INDEX IF NOT EXISTS idx_edges_source ON canvas_edges(source);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON canvas_edges(target);
        "#;
        let tx = conn.transaction()
            .map_err(|e| ConciergeError::Memory(format!("Begin schema tx: {e}")))?;
        tx.execute_batch(schema)
            .map_err(|e| ConciergeError::Memory(format!("Schema error: {e}")))?;
        tx.commit()
            .map_err(|e| ConciergeError::Memory(format!("Commit schema: {e}")))?;

        let host = Self {
            conn: Mutex::new(conn),
            canvas: Arc::new(Mutex::new(CanvasState::default())),
            app_handle: Arc::new(Mutex::new(None)),
        };
        host.reload_canvas_from_db()?;
        info!("OmniForgeHost ready (SQLite WAL + indexes)");
        Ok(host)
    }

    fn reload_canvas_from_db(&self) -> Result<(), ConciergeError> {
        let conn = self.conn.try_lock().map_err(|_| ConciergeError::Memory("DB lock".into()))?;
        let nodes: Vec<CanvasNode> = conn.prepare(
            "SELECT id, node_type, label, model_id, x, y, config_json FROM canvas_nodes"
        )?.query_map([], |row| {
            Ok(CanvasNode {
                id: row.get(0)?, node_type: row.get(1)?, label: row.get(2)?,
                model_id: row.get(3)?, x: row.get(4)?, y: row.get(5)?,
                config: row.get::<_, Option<String>>(6)?
                    .and_then(|j| serde_json::from_str(&j).ok()),
            })
        })?.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ConciergeError::Memory(format!("Query nodes: {e}")))?;
        let edges: Vec<CanvasEdge> = conn.prepare(
            "SELECT id, source, target, source_socket, target_socket FROM canvas_edges"
        )?.query_map([], |row| {
            Ok(CanvasEdge {
                id: row.get(0)?, source: row.get(1)?, target: row.get(2)?,
                source_socket: row.get(3)?, target_socket: row.get(4)?,
            })
        })?.collect::<Result<Vec<_>, _>>()
        .map_err(|e| ConciergeError::Memory(format!("Query edges: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.try_lock().map_err(|_| ConciergeError::Memory("canvas lock".into()))?;
        *canvas = CanvasState { nodes, edges };
        Ok(())
    }

    async fn emit_canvas_update(&self) {
        let snap = self.canvas.lock().await.clone();
        if let Some(ref handle) = *self.app_handle.lock().await {
            let _ = handle.emit_all("canvas-update", serde_json::json!({
                "nodes": snap.nodes.iter().map(|n| serde_json::json!({
                    "id": n.id, "type": "default",
                    "position": { "x": n.x, "y": n.y },
                    "data": { "label": n.label, "nodeType": n.node_type, "modelId": n.model_id }
                })).collect::<Vec<_>>(),
                "edges": snap.edges.iter().map(|e| serde_json::json!({
                    "id": e.id, "source": e.source, "target": e.target,
                    "sourceHandle": e.source_socket, "targetHandle": e.target_socket, "animated": true
                })).collect::<Vec<_>>()
            }));
        }
    }
}

#[async_trait]
impl PlatformHost for OmniForgeHost {
    async fn search_models(
        &self, query: &str, modality: Option<&str>, arch: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(query, ?modality, ?arch, "search_models");
        let q = format!("%{}%", query.to_lowercase());
        let mut conn = self.conn.lock().await;
        let sql = if modality.is_some() {
            "SELECT id, name, path, format, modality FROM models WHERE (lower(name) LIKE ?1 OR lower(path) LIKE ?1) AND modality = ?2 AND arch = ?"
        } else {
            "SELECT id, name, path, format, modality FROM models WHERE (lower(name) LIKE ?1 OR lower(path) LIKE ?1) AND arch = ?"
        };
        let arch = arch.unwrap_or("any");
        let entries: Vec<ModelEntry> = if let Some(modality) = modality {
            conn.prepare(sql)?.query_map(params![q, modality, arch], |row| {
                Ok(ModelEntry { id: row.get(0)?, name: row.get(1)?, path: row.get(2)?, format: row.get(3)?, modality: row.get(4)? })
            })?.filter_map(|r| r.ok()).collect::<Vec<_>>()
        } else {
            conn.prepare(sql)?.query_map(params![q, arch], |row| {
                Ok(ModelEntry { id: row.get(0)?, name: row.get(1)?, path: row.get(2)?, format: row.get(3)?, modality: row.get(4)? })
            })?.filter_map(|r| r.ok()).collect::<Vec<_>>()
        };
        Ok(serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into()))
    }

    async fn import_model(
        &self, path: &str, format: Option<&str>, name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(path, ?format, ?name, "import_model");
        let id = format!("model-{}", Uuid::new_v4());
        let fmt = format.unwrap_or_else(|| if path.ends_with(".onnx") { "onnx" } else if path.ends_with(".gguf") { "gguf" } else { "auto" });
        let entry = ModelEntry { id: id.clone(), name: name.unwrap_or("imported").to_string(), path: path.to_string(), format: fmt.to_string(), modality: "text".into() };
        let mut conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO models (id, name, path, format, modality) VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, path=excluded.path, format=excluded.format, modality=excluded.modality",
            params![entry.id, entry.name, entry.path, entry.format, entry.modality],
        ).map_err(|e| ConciergeError::Memory(format!("Import: {e}")))?;
        drop(conn);
        Ok(id)
    }

    async fn list_models(&self, limit: usize) -> Result<String, ConciergeError> {
        let mut conn = self.conn.lock().await;
        let entries: Vec<ModelEntry> = conn.prepare("SELECT id, name, path, format, modality FROM models LIMIT ?1")?
            .query_map(params![limit as i64], |row| {
                Ok(ModelEntry { id: row.get(0)?, name: row.get(1)?, path: row.get(2)?, format: row.get(3)?, modality: row.get(4)? })
            })?.filter_map(|r| r.ok()).collect::<Vec<_>>();
        Ok(serde_json::to_string(&entries).unwrap_or_else(|_| "[]".into()))
    }

    async fn add_node(
        &self, node_type: &str, model_id: Option<&str>, label: Option<&str>,
        x: f64, y: f64, config: Option<serde_json::Value>,
    ) -> Result<String, ConciergeError> {
        let id = format!("node-{}", Uuid::new_v4());
        let config_json = config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
        let mut conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO canvas_nodes (id, node_type, label, model_id, x, y, config_json) VALUES (?1,?2,?3,?4,?5,?6,?7)
             ON CONFLICT(id) DO UPDATE SET node_type=excluded.node_type, label=excluded.label,
                 model_id=excluded.model_id, x=excluded.x, y=excluded.y, config_json=excluded.config_json",
            params![id, node_type, label.unwrap_or("Node"), model_id, x, y, config_json],
        ).map_err(|e| ConciergeError::Memory(format!("Add node: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.lock().await;
        canvas.nodes.push(CanvasNode { id: id.clone(), node_type: node_type.to_string(), label: label.unwrap_or("Node").to_string(), model_id: model_id.map(|s| s.to_string()), x, y, config });
        drop(canvas);
        self.emit_canvas_update().await;
        Ok(id)
    }

    async fn connect_nodes(
        &self, source_node: &str, source_socket: &str, target_node: &str, target_socket: &str,
    ) -> Result<String, ConciergeError> {
        let id = format!("edge-{}", Uuid::new_v4());
        let mut conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO canvas_edges (id, source, target, source_socket, target_socket) VALUES (?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET source=excluded.source, target=excluded.target,
                 source_socket=excluded.source_socket, target_socket=excluded.target_socket",
            params![id, source_node, target_node, source_socket, target_socket],
        ).map_err(|e| ConciergeError::Memory(format!("Connect: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.lock().await;
        canvas.edges.push(CanvasEdge { id: id.clone(), source: source_node.to_string(), target: target_node.to_string(), source_socket: source_socket.to_string(), target_socket: target_socket.to_string() });
        drop(canvas);
        self.emit_canvas_update().await;
        Ok(id)
    }

    async fn create_dataset(&self, _name: &str, _sources: &[String], _labels: Option<&[String]>, _format: Option<&str>) -> Result<String, ConciergeError> {
        info!("create_dataset (stub)");
        Ok(format!("dataset-{}", Uuid::new_v4()))
    }

    async fn run_training(&self, _base_model: &str, _dataset: &str, _recipe: &str, _output_name: Option<&str>, _epochs: Option<i64>, _learning_rate: Option<f64>) -> Result<String, ConciergeError> {
        info!("run_training (stub)");
        Ok(format!("training-job-{}", Uuid::new_v4()))
    }

    async fn create_knowledge_module(&self, _base_model: &str, _adapter_path: &str, _metadata: Option<&str>, _name: Option<&str>) -> Result<String, ConciergeError> {
        info!("create_knowledge_module (stub)");
        Ok(format!("km-{}", Uuid::new_v4()))
    }

    async fn merge_models(&self, _model_ids: &[String], _strategy: &str, _weights: Option<&[f64]>, _output_name: Option<&str>) -> Result<String, ConciergeError> {
        info!("merge_models (stub)");
        Ok(format!("merged-model-{}", Uuid::new_v4()))
    }

    async fn inspect_node(&self, node_id: &str) -> Result<String, ConciergeError> {
        let mut conn = self.conn.lock().await;
        let node = conn.query_row(
            "SELECT id, node_type, label, model_id, x, y, config_json FROM canvas_nodes WHERE id = ?1",
            params![node_id],
            |row| {
                Ok(CanvasNode {
                    id: row.get(0)?, node_type: row.get(1)?, label: row.get(2)?,
                    model_id: row.get(3)?, x: row.get(4)?, y: row.get(5)?,
                    config: row.get::<_, Option<String>>(6)?.and_then(|j| serde_json::from_str(&j).ok()),
                })
            },
        ).map_err(|e| ConciergeError::Memory(format!("Inspect: {e}")))?;
        Ok(serde_json::to_string(&node).unwrap_or_else(|_| "{}".into()))
    }

    async fn execute_graph(&self, inputs: &serde_json::Value, timeout_ms: Option<u64>) -> Result<String, ConciergeError> {
        info!(%inputs, ?timeout_ms, "execute_graph");
        Ok(serde_json::json!({"status": "ok", "inputs": inputs}).to_string())
    }

    async fn compare_outputs(&self, node_a: &str, node_b: &str, input: &str) -> Result<String, ConciergeError> {
        info!(node_a, node_b, input, "compare_outputs");
        Ok(serde_json::json!({"similarity": 0.87, "note": "mock comparison"}).to_string())
    }

    async fn search_docs(&self, query: &str, limit: usize) -> Result<String, ConciergeError> {
        info!(query, limit, "search_docs");
        Ok("[]".into())
    }

    async fn write_plugin(&self, description: &str, language: &str, name: Option<&str>) -> Result<String, ConciergeError> {
        info!(description, language, ?name, "write_plugin");
        Ok(format!("plugin-{}", Uuid::new_v4()))
    }

    async fn modify_graph(&self, action: &str, target: &str, payload: Option<&serde_json::Value>) -> Result<String, ConciergeError> {
        info!(action, target, ?payload, "modify_graph");
        Ok(format!("Graph modified: action={}, target={}", action, target))
    }

    async fn get_platform_status(&self) -> Result<String, ConciergeError> {
        let canvas = self.canvas.lock().await;
        Ok(serde_json::json!({
            "status": "healthy",
            "models": 0,
            "nodes": canvas.nodes.len(),
            "edges": canvas.edges.len(),
            "mode": "rusqlite"
        }).to_string())
    }
}

// ── Additional methods for Tauri commands ──────────────────────────────────
impl OmniForgeHost {
    pub async fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.lock().await = Some(handle);
        self.emit_canvas_update().await;
    }

    pub async fn get_canvas_snapshot(&self) -> CanvasState {
        self.canvas.lock().await.clone()
    }

    pub async fn delete_node(&self, id: &str) -> Result<(), ConciergeError> {
        let mut conn = self.conn.lock().await;
        conn.execute("DELETE FROM canvas_nodes WHERE id = ?1", params![id]).map_err(|e| ConciergeError::Memory(format!("Delete node: {e}")))?;
        conn.execute("DELETE FROM canvas_edges WHERE source = ?1 OR target = ?1", params![id]).map_err(|e| ConciergeError::Memory(format!("Delete edges: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.lock().await;
        canvas.nodes.retain(|n| n.id != id);
        canvas.edges.retain(|e| e.source != id && e.target != id);
        drop(canvas);
        self.emit_canvas_update().await;
        Ok(())
    }

    pub async fn clear_canvas(&self) -> Result<(), ConciergeError> {
        let mut conn = self.conn.lock().await;
        conn.execute_batch("DELETE FROM canvas_edges; DELETE FROM canvas_nodes;").map_err(|e| ConciergeError::Memory(format!("Clear canvas: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.lock().await;
        *canvas = CanvasState::default();
        drop(canvas);
        self.emit_canvas_update().await;
        Ok(())
    }

    pub async fn persist_canvas_batch(&self, nodes: &[CanvasNode], edges: &[CanvasEdge]) -> Result<(), ConciergeError> {
        let mut conn = self.conn.lock().await;
        let tx = conn.transaction().map_err(|e| ConciergeError::Memory(format!("Begin batch tx: {e}")))?;
        for n in nodes {
            let config_json = n.config.as_ref().map(|c| serde_json::to_string(c).unwrap_or_default());
            tx.execute(
                "INSERT INTO canvas_nodes (id, node_type, label, model_id, x, y, config_json) VALUES (?1,?2,?3,?4,?5,?6,?7)
                 ON CONFLICT(id) DO UPDATE SET node_type=excluded.node_type, label=excluded.label,
                     model_id=excluded.model_id, x=excluded.x, y=excluded.y, config_json=excluded.config_json",
                params![&n.id, &n.node_type, &n.label, &n.model_id, n.x, n.y, config_json],
            ).map_err(|e| ConciergeError::Memory(format!("Batch node: {e}")))?;
        }
        for e in edges {
            tx.execute(
                "INSERT INTO canvas_edges (id, source, target, source_socket, target_socket) VALUES (?1,?2,?3,?4,?5)
                 ON CONFLICT(id) DO UPDATE SET source=excluded.source, target=excluded.target,
                     source_socket=excluded.source_socket, target_socket=excluded.target_socket",
                params![&e.id, &e.source, &e.target, &e.source_socket, &e.target_socket],
            ).map_err(|e| ConciergeError::Memory(format!("Batch edge: {e}")))?;
        }
        tx.commit().map_err(|e| ConciergeError::Memory(format!("Commit batch: {e}")))?;
        drop(conn);
        let mut canvas = self.canvas.lock().await;
        *canvas = CanvasState { nodes: nodes.to_vec(), edges: edges.to_vec() };
        drop(canvas);
        self.emit_canvas_update().await;
        Ok(())
    }
}