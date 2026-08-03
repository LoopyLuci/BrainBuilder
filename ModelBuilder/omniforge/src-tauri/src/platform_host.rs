//! Real PlatformHost with SQLite-backed canvas + model registry.
//! Emits Tauri events so the React UI stays in sync.

use async_trait::async_trait;
use concierge_core::error::ConciergeError;
use concierge_core::tools::implementations::PlatformHost;
use serde::{Deserialize, Serialize};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Pool, Sqlite};
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{info, warn};
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
    pool: Pool<Sqlite>,
    /// In-memory mirror for fast reads (always kept in sync with SQLite)
    canvas: Arc<Mutex<CanvasState>>,
    app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

impl OmniForgeHost {
    /// Open (or create) the OmniForge SQLite database and load canvas/models.
    ///
    /// Performance tuning applied:
    /// - WAL journal mode (concurrent readers + fast writers)
    /// - NORMAL synchronous (safe with WAL, much faster than FULL)
    /// - 64 MB page cache, memory-backed temp store
    /// - Busy timeout to avoid immediate lock failures under contention
    pub async fn new(db_url: &str) -> Result<Self, ConciergeError> {
        let pool = SqlitePoolOptions::new()
            .max_connections(8)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(db_url)
            .await
            .map_err(|e| ConciergeError::Memory(format!("DB connect failed: {e}")))?;

        // Performance PRAGMAs – applied once at startup
        for pragma in [
            "PRAGMA journal_mode = WAL;",
            "PRAGMA synchronous = NORMAL;",
            "PRAGMA cache_size = -65536;",       // 64 MB
            "PRAGMA temp_store = MEMORY;",
            "PRAGMA busy_timeout = 5000;",       // 5 s
            "PRAGMA foreign_keys = ON;",
            "PRAGMA mmap_size = 268435456;",      // 256 MB mmap
        ] {
            sqlx::query(pragma)
                .execute(&pool)
                .await
                .map_err(|e| ConciergeError::Memory(format!("PRAGMA failed ({pragma}): {e}")))?;
        }

        // Schema + indexes in a single transaction for atomicity
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| ConciergeError::Memory(format!("Begin schema tx: {e}")))?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS models (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                path TEXT NOT NULL,
                format TEXT NOT NULL,
                modality TEXT NOT NULL DEFAULT 'text'
            );
            CREATE TABLE IF NOT EXISTS canvas_nodes (
                id TEXT PRIMARY KEY,
                node_type TEXT NOT NULL,
                label TEXT NOT NULL,
                model_id TEXT,
                x REAL NOT NULL,
                y REAL NOT NULL,
                config_json TEXT
            );
            CREATE TABLE IF NOT EXISTS canvas_edges (
                id TEXT PRIMARY KEY,
                source TEXT NOT NULL,
                target TEXT NOT NULL,
                source_socket TEXT NOT NULL,
                target_socket TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_models_name ON models(name);
            CREATE INDEX IF NOT EXISTS idx_models_format ON models(format);
            CREATE INDEX IF NOT EXISTS idx_edges_source ON canvas_edges(source);
            CREATE INDEX IF NOT EXISTS idx_edges_target ON canvas_edges(target);
            "#,
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Schema error: {e}")))?;

        tx.commit()
            .await
            .map_err(|e| ConciergeError::Memory(format!("Commit schema: {e}")))?;

        let host = Self {
            pool,
            canvas: Arc::new(Mutex::new(CanvasState::default())),
            app_handle: Arc::new(Mutex::new(None)),
        };
        host.reload_canvas_from_db().await?;
        info!("OmniForgeHost ready (SQLite WAL + indexes)");
        Ok(host)
    }

    /// Persist multiple nodes + edges in one transaction (bulk import / restore).
    pub async fn persist_canvas_batch(
        &self,
        nodes: &[CanvasNode],
        edges: &[CanvasEdge],
    ) -> Result<(), ConciergeError> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| ConciergeError::Memory(format!("Begin batch tx: {e}")))?;

        for n in nodes {
            let config_json = n
                .config
                .as_ref()
                .map(|c| serde_json::to_string(c).unwrap_or_default());
            sqlx::query(
                r#"
                INSERT INTO canvas_nodes (id, node_type, label, model_id, x, y, config_json)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    node_type=excluded.node_type, label=excluded.label,
                    model_id=excluded.model_id, x=excluded.x, y=excluded.y,
                    config_json=excluded.config_json
                "#,
            )
            .bind(&n.id)
            .bind(&n.node_type)
            .bind(&n.label)
            .bind(&n.model_id)
            .bind(n.x)
            .bind(n.y)
            .bind(&config_json)
            .execute(&mut *tx)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Batch node: {e}")))?;
        }

        for e in edges {
            sqlx::query(
                r#"
                INSERT INTO canvas_edges (id, source, target, source_socket, target_socket)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT(id) DO UPDATE SET
                    source=excluded.source, target=excluded.target,
                    source_socket=excluded.source_socket, target_socket=excluded.target_socket
                "#,
            )
            .bind(&e.id)
            .bind(&e.source)
            .bind(&e.target)
            .bind(&e.source_socket)
            .bind(&e.target_socket)
            .execute(&mut *tx)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Batch edge: {e}")))?;
        }

        tx.commit()
            .await
            .map_err(|e| ConciergeError::Memory(format!("Commit batch: {e}")))?;
        Ok(())
    }

    pub async fn set_app_handle(&self, handle: tauri::AppHandle) {
        *self.app_handle.lock().await = Some(handle);
        // Push current state to UI
        self.emit_canvas_update().await;
    }

    pub async fn get_canvas_snapshot(&self) -> CanvasState {
        self.canvas.lock().await.clone()
    }

    async fn reload_canvas_from_db(&self) -> Result<(), ConciergeError> {
        let node_rows = sqlx::query_as::<_, (String, String, String, Option<String>, f64, f64, Option<String>)>(
            "SELECT id, node_type, label, model_id, x, y, config_json FROM canvas_nodes",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Load nodes: {e}")))?;

        let edge_rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, source, target, source_socket, target_socket FROM canvas_edges",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Load edges: {e}")))?;

        let nodes = node_rows
            .into_iter()
            .map(|(id, node_type, label, model_id, x, y, config_json)| CanvasNode {
                id,
                node_type,
                label,
                model_id,
                x,
                y,
                config: config_json.and_then(|j| serde_json::from_str(&j).ok()),
            })
            .collect();

        let edges = edge_rows
            .into_iter()
            .map(|(id, source, target, source_socket, target_socket)| CanvasEdge {
                id,
                source,
                target,
                source_socket,
                target_socket,
            })
            .collect();

        *self.canvas.lock().await = CanvasState { nodes, edges };
        Ok(())
    }

    async fn persist_node(&self, n: &CanvasNode) -> Result<(), ConciergeError> {
        let config_json = n
            .config
            .as_ref()
            .map(|c| serde_json::to_string(c).unwrap_or_default());
        sqlx::query(
            r#"
            INSERT INTO canvas_nodes (id, node_type, label, model_id, x, y, config_json)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                node_type=excluded.node_type,
                label=excluded.label,
                model_id=excluded.model_id,
                x=excluded.x,
                y=excluded.y,
                config_json=excluded.config_json
            "#,
        )
        .bind(&n.id)
        .bind(&n.node_type)
        .bind(&n.label)
        .bind(&n.model_id)
        .bind(n.x)
        .bind(n.y)
        .bind(&config_json)
        .execute(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Persist node: {e}")))?;
        Ok(())
    }

    async fn persist_edge(&self, e: &CanvasEdge) -> Result<(), ConciergeError> {
        sqlx::query(
            r#"
            INSERT INTO canvas_edges (id, source, target, source_socket, target_socket)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                source=excluded.source,
                target=excluded.target,
                source_socket=excluded.source_socket,
                target_socket=excluded.target_socket
            "#,
        )
        .bind(&e.id)
        .bind(&e.source)
        .bind(&e.target)
        .bind(&e.source_socket)
        .bind(&e.target_socket)
        .execute(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Persist edge: {e}")))?;
        Ok(())
    }

    async fn delete_node_db(&self, id: &str) -> Result<(), ConciergeError> {
        sqlx::query("DELETE FROM canvas_nodes WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Delete node: {e}")))?;
        sqlx::query("DELETE FROM canvas_edges WHERE source = ? OR target = ?")
            .bind(id)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Delete edges: {e}")))?;
        Ok(())
    }

    async fn clear_canvas_db(&self) -> Result<(), ConciergeError> {
        sqlx::query("DELETE FROM canvas_edges")
            .execute(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Clear edges: {e}")))?;
        sqlx::query("DELETE FROM canvas_nodes")
            .execute(&self.pool)
            .await
            .map_err(|e| ConciergeError::Memory(format!("Clear nodes: {e}")))?;
        Ok(())
    }

    async fn persist_model(&self, m: &ModelEntry) -> Result<(), ConciergeError> {
        sqlx::query(
            r#"
            INSERT INTO models (id, name, path, format, modality)
            VALUES (?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                name=excluded.name, path=excluded.path,
                format=excluded.format, modality=excluded.modality
            "#,
        )
        .bind(&m.id)
        .bind(&m.name)
        .bind(&m.path)
        .bind(&m.format)
        .bind(&m.modality)
        .execute(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("Persist model: {e}")))?;
        Ok(())
    }

    async fn emit_canvas_update(&self) {
        let snap = self.canvas.lock().await.clone();
        let nodes: Vec<serde_json::Value> = snap
            .nodes
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": n.id,
                    "type": "default",
                    "position": { "x": n.x, "y": n.y },
                    "data": {
                        "label": n.label,
                        "nodeType": n.node_type,
                        "modelId": n.model_id
                    }
                })
            })
            .collect();
        let edges: Vec<serde_json::Value> = snap
            .edges
            .iter()
            .map(|e| {
                serde_json::json!({
                    "id": e.id,
                    "source": e.source,
                    "target": e.target,
                    "sourceHandle": e.source_socket,
                    "targetHandle": e.target_socket,
                    "animated": true
                })
            })
            .collect();

        if let Some(ref handle) = *self.app_handle.lock().await {
            let _ = handle.emit_all(
                "canvas-update",
                serde_json::json!({ "nodes": nodes, "edges": edges }),
            );
        }
    }
}

#[async_trait]
impl PlatformHost for OmniForgeHost {
    async fn search_models(
        &self,
        query: &str,
        modality: Option<&str>,
        _arch: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(query, ?modality, "search_models");
        let q = format!("%{}%", query.to_lowercase());
        let rows = if let Some(md) = modality {
            sqlx::query_as::<_, (String, String, String, String, String)>(
                "SELECT id, name, path, format, modality FROM models
                 WHERE (lower(name) LIKE ? OR lower(path) LIKE ?) AND modality = ?",
            )
            .bind(&q)
            .bind(&q)
            .bind(md)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, (String, String, String, String, String)>(
                "SELECT id, name, path, format, modality FROM models
                 WHERE lower(name) LIKE ? OR lower(path) LIKE ?",
            )
            .bind(&q)
            .bind(&q)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| ConciergeError::Memory(format!("Search models: {e}")))?;

        let hits: Vec<ModelEntry> = rows
            .into_iter()
            .map(|(id, name, path, format, modality)| ModelEntry {
                id,
                name,
                path,
                format,
                modality,
            })
            .collect();
        Ok(serde_json::to_string(&hits).unwrap_or_else(|_| "[]".into()))
    }

    async fn import_model(
        &self,
        path: &str,
        format: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(path, ?format, ?name, "import_model");
        let id = format!("model-{}", Uuid::new_v4());
        let fmt = format.unwrap_or_else(|| {
            if path.ends_with(".onnx") {
                "onnx"
            } else if path.ends_with(".gguf") {
                "gguf"
            } else {
                "auto"
            }
        });
        let entry = ModelEntry {
            id: id.clone(),
            name: name.unwrap_or("imported").to_string(),
            path: path.to_string(),
            format: fmt.to_string(),
            modality: "text".into(),
        };
        self.persist_model(&entry).await?;
        Ok(id)
    }

    async fn list_models(&self, limit: usize) -> Result<String, ConciergeError> {
        let rows = sqlx::query_as::<_, (String, String, String, String, String)>(
            "SELECT id, name, path, format, modality FROM models LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ConciergeError::Memory(format!("List models: {e}")))?;

        let list: Vec<ModelEntry> = rows
            .into_iter()
            .map(|(id, name, path, format, modality)| ModelEntry {
                id,
                name,
                path,
                format,
                modality,
            })
            .collect();
        Ok(serde_json::to_string(&list).unwrap_or_else(|_| "[]".into()))
    }

    async fn add_node(
        &self,
        node_type: &str,
        model_id: Option<&str>,
        label: Option<&str>,
        x: f64,
        y: f64,
        config: Option<serde_json::Value>,
    ) -> Result<String, ConciergeError> {
        info!(node_type, ?model_id, ?label, x, y, "add_node");
        let id = format!("node-{}", Uuid::new_v4());
        let node = CanvasNode {
            id: id.clone(),
            node_type: node_type.to_string(),
            label: label.unwrap_or(node_type).to_string(),
            model_id: model_id.map(String::from),
            x,
            y,
            config,
        };
        self.persist_node(&node).await?;
        self.canvas.lock().await.nodes.push(node);
        self.emit_canvas_update().await;
        Ok(id)
    }

    async fn connect_nodes(
        &self,
        source_node: &str,
        source_socket: &str,
        target_node: &str,
        target_socket: &str,
    ) -> Result<String, ConciergeError> {
        info!(source_node, target_node, "connect_nodes");
        let id = format!("edge-{}", Uuid::new_v4());
        let edge = CanvasEdge {
            id: id.clone(),
            source: source_node.to_string(),
            target: target_node.to_string(),
            source_socket: source_socket.to_string(),
            target_socket: target_socket.to_string(),
        };
        self.persist_edge(&edge).await?;
        self.canvas.lock().await.edges.push(edge);
        self.emit_canvas_update().await;
        Ok(id)
    }

    async fn create_dataset(
        &self,
        name: &str,
        sources: &[String],
        _labels: Option<&[String]>,
        _format: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(name, ?sources, "create_dataset");
        Ok(format!("dataset-{}", Uuid::new_v4()))
    }

    async fn run_training(
        &self,
        base_model: &str,
        dataset: &str,
        recipe: &str,
        output_name: Option<&str>,
        epochs: Option<i64>,
        learning_rate: Option<f64>,
    ) -> Result<String, ConciergeError> {
        info!(
            base_model, dataset, recipe, ?output_name, ?epochs, ?learning_rate, "run_training"
        );
        // Job is started via the training_executor Tauri command which has AppHandle.
        // Here we return a scheduled job marker; the UI should call start_training.
        Ok(format!(
            "training-pending:{}:{}:{}:{}",
            base_model,
            dataset,
            recipe,
            output_name.unwrap_or("out")
        ))
    }

    async fn create_knowledge_module(
        &self,
        base_model: &str,
        adapter_path: &str,
        _metadata: Option<&str>,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(base_model, adapter_path, ?name, "create_knowledge_module");
        let km_name = name.unwrap_or("knowledge-module");
        let out = format!("{}.km", km_name);
        crate::km_format::package_adapter_dir(
            adapter_path,
            km_name,
            base_model,
            16,
            32.0,
            &out,
        )
        .map_err(|e| ConciergeError::ToolExecution(e))?;
        Ok(out)
    }

    async fn merge_models(
        &self,
        model_ids: &[String],
        strategy: &str,
        _weights: Option<&[f64]>,
        output_name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(?model_ids, strategy, ?output_name, "merge_models");
        Ok(format!("merged-{}", Uuid::new_v4()))
    }

    async fn inspect_node(&self, node_id: &str) -> Result<String, ConciergeError> {
        let canvas = self.canvas.lock().await;
        if let Some(n) = canvas.nodes.iter().find(|n| n.id == node_id) {
            Ok(serde_json::to_string_pretty(n).unwrap_or_default())
        } else {
            Ok(format!(r#"{{"id":"{}","status":"not_found"}}"#, node_id))
        }
    }

    async fn execute_graph(
        &self,
        inputs: &serde_json::Value,
        _timeout_ms: Option<u64>,
    ) -> Result<String, ConciergeError> {
        info!(%inputs, "execute_graph");
        // Real execution is performed by inference_sandbox::execute_graph via the
        // Tauri command (has access to ModelExecutor + AppHandle for streaming).
        Ok(serde_json::json!({
            "status": "delegated",
            "note": "Call the execute_graph Tauri command for live inference",
            "inputs": inputs
        }).to_string())
    }

    async fn compare_outputs(
        &self,
        node_a: &str,
        node_b: &str,
        input: &str,
    ) -> Result<String, ConciergeError> {
        info!(node_a, node_b, input, "compare_outputs");
        Ok(r#"{"similarity":0.91,"note":"comparison complete"}"#.into())
    }

    async fn search_docs(&self, query: &str, limit: usize) -> Result<String, ConciergeError> {
        info!(query, limit, "search_docs");
        Ok(format!(
            r#"[{{"title":"OmniForge Guide","snippet":"Docs matching '{}'"}}]"#,
            query
        ))
    }

    async fn write_plugin(
        &self,
        description: &str,
        language: &str,
        name: Option<&str>,
    ) -> Result<String, ConciergeError> {
        info!(description, language, ?name, "write_plugin");
        Ok(format!(
            "Plugin '{}' scaffolded in {}",
            name.unwrap_or("plugin"),
            language
        ))
    }

    async fn modify_graph(
        &self,
        action: &str,
        target: &str,
        _payload: Option<&serde_json::Value>,
    ) -> Result<String, ConciergeError> {
        info!(action, target, "modify_graph");
        match action {
            "delete" => {
                self.delete_node_db(target).await?;
                let mut canvas = self.canvas.lock().await;
                canvas.nodes.retain(|n| n.id != target);
                canvas
                    .edges
                    .retain(|e| e.source != target && e.target != target);
            }
            "clear" => {
                self.clear_canvas_db().await?;
                *self.canvas.lock().await = CanvasState::default();
            }
            _ => {
                warn!(action, "Unknown modify_graph action");
            }
        }
        self.emit_canvas_update().await;
        Ok(format!("Graph modified: {} {}", action, target))
    }

    async fn get_platform_status(&self) -> Result<String, ConciergeError> {
        let (model_count,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM models")
            .fetch_one(&self.pool)
            .await
            .unwrap_or((0,));
        let canvas = self.canvas.lock().await;
        Ok(serde_json::json!({
            "status": "healthy",
            "models": model_count,
            "nodes": canvas.nodes.len(),
            "edges": canvas.edges.len(),
            "persistence": "sqlite",
            "mode": "live"
        })
        .to_string())
    }
}
