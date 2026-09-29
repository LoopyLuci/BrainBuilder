use std::sync::Arc;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, Result as SqliteResult};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: String,
    pub kind: String,
    pub content: String,
    pub embedding: Option<Vec<f64>>,
    pub importance: f64,
    pub created_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub access_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskPlan {
    pub id: String,
    pub title: String,
    pub description: String,
    pub status: String,
    pub steps: Vec<String>,
    pub current_step: usize,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub result: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reflection {
    pub id: String,
    pub plan_id: Option<String>,
    pub what_went_well: String,
    pub what_failed: String,
    pub lessons_learned: String,
    pub score: f64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    pub key: String,
    pub value: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: String,
    pub event_type: String,
    pub details: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolEntry {
    pub name: String,
    pub description: String,
    pub schema: String,
    pub enabled: bool,
    pub success_count: i64,
    pub failure_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LuciTool {
    pub name: String,
    pub description: String,
    pub schema: serde_json::Value,
    pub enabled: bool,
    pub success_count: i64,
    pub failure_count: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEntry {
    pub id: String,
    pub prompt: String,
    pub score: f64,
    pub generation: i64,
    pub parent_id: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDefinition {
    pub id: String,
    pub name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub params: serde_json::Value,
    pub implementation: String,
    pub source: String,
    pub confidence: f64,
    pub success_count: i64,
    pub failure_count: i64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCase {
    pub id: String,
    pub title: String,
    pub description: String,
    pub input_example: serde_json::Value,
    pub output_example: serde_json::Value,
    pub tags: Vec<String>,
    pub source: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelRecord {
    pub id: String,
    pub name: String,
    pub source: String,
    pub model_type: String,
    pub format: String,
    pub path: Option<String>,
    pub url: Option<String>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrainingJob {
    pub id: String,
    pub model_id: String,
    pub mode: String,
    pub dataset_ids: Vec<String>,
    pub status: String,
    pub metrics: serde_json::Value,
    pub artifact_path: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatasetRecord {
    pub id: String,
    pub name: String,
    pub source: String,
    pub size: usize,
    pub format: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug)]
pub struct LuciStore {
    conn: Arc<Mutex<Connection>>,
}

impl LuciStore {
    pub async fn new<P: AsRef<std::path::Path>>(path: P) -> SqliteResult<Self> {
        std::fs::create_dir_all(path.as_ref().parent().unwrap_or_else(|| std::path::Path::new("."))).map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))?;
        let conn = Connection::open(path.as_ref())?;
        let store = Self { conn: Arc::new(Mutex::new(conn)) };
        store.migrate().await?;
        Ok(store)
    }

    async fn migrate(&self) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS memories (id TEXT PRIMARY KEY, kind TEXT, content TEXT, embedding TEXT, importance REAL, created_at TEXT, accessed_at TEXT, access_count INTEGER);
             CREATE TABLE IF NOT EXISTS plans (id TEXT PRIMARY KEY, title TEXT, description TEXT, status TEXT, steps TEXT, current_step INTEGER, created_at TEXT, updated_at TEXT, completed_at TEXT, result TEXT);
             CREATE TABLE IF NOT EXISTS reflections (id TEXT PRIMARY KEY, plan_id TEXT, what_went_well TEXT, what_failed TEXT, lessons_learned TEXT, score REAL, created_at TEXT);
             CREATE TABLE IF NOT EXISTS user_profile (key TEXT PRIMARY KEY, value TEXT, updated_at TEXT);
             CREATE TABLE IF NOT EXISTS audit (id TEXT PRIMARY KEY, event_type TEXT, details TEXT, created_at TEXT);
             CREATE TABLE IF NOT EXISTS tools (name TEXT PRIMARY KEY, description TEXT, schema TEXT, enabled INTEGER, success_count INTEGER, failure_count INTEGER);
             CREATE TABLE IF NOT EXISTS prompts (id TEXT PRIMARY KEY, prompt TEXT, score REAL, generation INTEGER, parent_id TEXT, created_at TEXT);
             CREATE TABLE IF NOT EXISTS skills (id TEXT PRIMARY KEY, name TEXT, description TEXT, tags TEXT, params TEXT, implementation TEXT, source TEXT, confidence REAL, success_count INTEGER, failure_count INTEGER, created_at TEXT, updated_at TEXT);
             CREATE TABLE IF NOT EXISTS task_cases (id TEXT PRIMARY KEY, title TEXT, description TEXT, input_example TEXT, output_example TEXT, tags TEXT, source TEXT, created_at TEXT);
             CREATE TABLE IF NOT EXISTS models (id TEXT PRIMARY KEY, name TEXT, source TEXT, model_type TEXT, format TEXT, path TEXT, url TEXT, metadata TEXT, created_at TEXT);
             CREATE TABLE IF NOT EXISTS datasets (id TEXT PRIMARY KEY, name TEXT, source TEXT, size INTEGER, format TEXT, metadata TEXT, created_at TEXT);
             CREATE TABLE IF NOT EXISTS training_jobs (id TEXT PRIMARY KEY, model_id TEXT, mode TEXT, dataset_ids TEXT, status TEXT, metrics TEXT, artifact_path TEXT, created_at TEXT, updated_at TEXT);"
        )?;
        Ok(())
    }

    // --- Memory ---
    pub async fn remember(&self, entry: MemoryEntry) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO memories (id, kind, content, embedding, importance, created_at, accessed_at, access_count) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, COALESCE((SELECT access_count FROM memories WHERE id=?1), 0))",
            params![entry.id, entry.kind, entry.content, entry.embedding.map(|v| serde_json::to_string(&v).unwrap()), entry.importance, entry.created_at.to_rfc3339(), entry.accessed_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn recall(&self, kind: Option<&str>, limit: usize) -> SqliteResult<Vec<MemoryEntry>> {
        let conn = self.conn.lock().await;
        let mut stmt = if let Some(_k) = kind {
            conn.prepare("SELECT id, kind, content, embedding, importance, created_at, accessed_at, access_count FROM memories WHERE kind = ?1 ORDER BY importance DESC, accessed_at DESC LIMIT ?2")?
        } else {
            conn.prepare("SELECT id, kind, content, embedding, importance, created_at, accessed_at, access_count FROM memories ORDER BY importance DESC, accessed_at DESC LIMIT ?1")?
        };

        let rows = if let Some(k) = kind {
            stmt.query_map(params![k, limit as i64], memory_row)?
        } else {
            stmt.query_map(params![limit as i64], memory_row)?
        };

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub async fn forget(&self, id: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute("DELETE FROM memories WHERE id = ?1", params![id])?;
        Ok(())
    }

    // --- Plans ---
    pub async fn save_plan(&self, plan: TaskPlan) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO plans (id, title, description, status, steps, current_step, created_at, updated_at, completed_at, result) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            params![plan.id, plan.title, plan.description, plan.status, serde_json::to_string(&plan.steps).unwrap(), plan.current_step as i64, plan.created_at.to_rfc3339(), plan.updated_at.to_rfc3339(), plan.completed_at.map(|t| t.to_rfc3339()), plan.result]
        )?;
        Ok(())
    }

    pub async fn list_plans(&self, status: Option<&str>) -> SqliteResult<Vec<TaskPlan>> {
        let conn = self.conn.lock().await;
        let mut stmt = if let Some(s) = status {
            conn.prepare("SELECT id, title, description, status, steps, current_step, created_at, updated_at, completed_at, result FROM plans WHERE status = ?1 ORDER BY updated_at DESC")?
        } else {
            conn.prepare("SELECT id, title, description, status, steps, current_step, created_at, updated_at, completed_at, result FROM plans ORDER BY updated_at DESC")?
        };

        let rows = if let Some(s) = status {
            stmt.query_map(params![s], plan_row)?
        } else {
            stmt.query_map([], plan_row)?
        };

        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Reflections ---
    pub async fn save_reflection(&self, reflection: Reflection) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO reflections (id, plan_id, what_went_well, what_failed, lessons_learned, score, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![reflection.id, reflection.plan_id, reflection.what_went_well, reflection.what_failed, reflection.lessons_learned, reflection.score, reflection.created_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn recent_reflections(&self, limit: usize) -> SqliteResult<Vec<Reflection>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, plan_id, what_went_well, what_failed, lessons_learned, score, created_at FROM reflections ORDER BY created_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], reflection_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- User profile ---
    pub async fn set_preference(&self, key: &str, value: &str) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute("INSERT OR REPLACE INTO user_profile (key, value, updated_at) VALUES (?1, ?2, ?3)", params![key, value, Utc::now().to_rfc3339()])?;
        Ok(())
    }

    pub async fn get_preference(&self, key: &str) -> SqliteResult<Option<String>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT value FROM user_profile WHERE key = ?1")?;
        let mut rows = stmt.query_map(params![key], |row| row.get(0))?;
        match rows.next() {
            Some(v) => Ok(Some(v?)),
            None => Ok(None),
        }
    }

    // --- Tools ---
    pub async fn register_tool(&self, name: &str, description: &str, schema: serde_json::Value) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO tools (name, description, schema, enabled, success_count, failure_count) VALUES (?1, ?2, ?3, ?4, COALESCE((SELECT success_count FROM tools WHERE name=?1), 0), COALESCE((SELECT failure_count FROM tools WHERE name=?1), 0))",
            params![name, description, schema.to_string(), true]
        )?;
        Ok(())
    }

    pub async fn list_tools(&self) -> SqliteResult<Vec<ToolEntry>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT name, description, schema, enabled, success_count, failure_count FROM tools")?;
        let rows = stmt.query_map([], tool_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Prompts ---
    pub async fn save_prompt(&self, prompt: &str, score: f64, generation: i64, parent_id: Option<&str>) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO prompts (id, prompt, score, generation, parent_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![uuid::Uuid::new_v4().to_string(), prompt, score, generation, parent_id, Utc::now().to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn best_prompts(&self, limit: usize) -> SqliteResult<Vec<(String, f64, i64)>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT prompt, score, generation FROM prompts ORDER BY score DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], prompt_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Skills ---
    pub async fn save_skill(&self, skill: SkillDefinition) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO skills (id, name, description, tags, params, implementation, source, confidence, success_count, failure_count, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, COALESCE((SELECT success_count FROM skills WHERE id=?1), 0), COALESCE((SELECT failure_count FROM skills WHERE id=?1), 0), ?9, ?10)",
            params![skill.id, skill.name, skill.description, serde_json::to_string(&skill.tags).unwrap(), skill.params.to_string(), skill.implementation, skill.source, skill.confidence, skill.created_at.to_rfc3339(), Utc::now().to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn list_skills(&self, limit: usize) -> SqliteResult<Vec<SkillDefinition>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, description, tags, params, implementation, source, confidence, success_count, failure_count, created_at, updated_at FROM skills ORDER BY confidence DESC, updated_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], skill_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    pub async fn update_skill_stats(&self, id: &str, success: bool) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        if success {
            conn.execute("UPDATE skills SET success_count = success_count + 1, updated_at = ?1 WHERE id = ?2", params![Utc::now().to_rfc3339(), id])?;
        } else {
            conn.execute("UPDATE skills SET failure_count = failure_count + 1, updated_at = ?1 WHERE id = ?2", params![Utc::now().to_rfc3339(), id])?;
        }
        Ok(())
    }

    // --- Task cases ---
    pub async fn add_task_case(&self, case: TaskCase) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO task_cases (id, title, description, input_example, output_example, tags, source, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            params![case.id, case.title, case.description, case.input_example.to_string(), case.output_example.to_string(), serde_json::to_string(&case.tags).unwrap(), case.source, case.created_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn list_task_cases(&self, limit: usize) -> SqliteResult<Vec<TaskCase>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, title, description, input_example, output_example, tags, source, created_at FROM task_cases ORDER BY created_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], task_case_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Model records ---
    pub async fn register_model(&self, model: ModelRecord) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO models (id, name, source, model_type, format, path, url, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![model.id, model.name, model.source, model.model_type, model.format, model.path, model.url, model.metadata.to_string(), model.created_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn list_models(&self) -> SqliteResult<Vec<ModelRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, source, model_type, format, path, url, metadata, created_at FROM models ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], model_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Datasets ---
    pub async fn register_dataset(&self, dataset: DatasetRecord) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO datasets (id, name, source, size, format, metadata, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![dataset.id, dataset.name, dataset.source, dataset.size as i64, dataset.format, dataset.metadata.to_string(), dataset.created_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn list_datasets(&self) -> SqliteResult<Vec<DatasetRecord>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, name, source, size, format, metadata, created_at FROM datasets ORDER BY created_at DESC")?;
        let rows = stmt.query_map([], dataset_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Training jobs ---
    pub async fn save_training_job(&self, job: TrainingJob) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT OR REPLACE INTO training_jobs (id, model_id, mode, dataset_ids, status, metrics, artifact_path, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![job.id, job.model_id, job.mode, serde_json::to_string(&job.dataset_ids).unwrap(), job.status, job.metrics.to_string(), job.artifact_path, job.created_at.to_rfc3339(), job.updated_at.to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn list_training_jobs(&self) -> SqliteResult<Vec<TrainingJob>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, model_id, mode, dataset_ids, status, metrics, artifact_path, created_at, updated_at FROM training_jobs ORDER BY updated_at DESC")?;
        let rows = stmt.query_map([], training_job_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }

    // --- Audit ---
    pub async fn audit(&self, event_type: &str, details: serde_json::Value) -> SqliteResult<()> {
        let conn = self.conn.lock().await;
        conn.execute(
            "INSERT INTO audit (id, event_type, details, created_at) VALUES (?1, ?2, ?3, ?4)",
            params![uuid::Uuid::new_v4().to_string(), event_type, details.to_string(), Utc::now().to_rfc3339()]
        )?;
        Ok(())
    }

    pub async fn recent_audit(&self, limit: usize) -> SqliteResult<Vec<AuditEvent>> {
        let conn = self.conn.lock().await;
        let mut stmt = conn.prepare("SELECT id, event_type, details, created_at FROM audit ORDER BY created_at DESC LIMIT ?1")?;
        let rows = stmt.query_map(params![limit as i64], audit_row)?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row?);
        }
        Ok(out)
    }
}

fn parse_dt(s: &str) -> Result<DateTime<Utc>, rusqlite::Error> {
    DateTime::parse_from_rfc3339(s)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| rusqlite::Error::FromSqlConversionFailure(0, rusqlite::types::Type::Text, Box::new(e)))
}

fn memory_row(row: &rusqlite::Row) -> rusqlite::Result<MemoryEntry> {
    Ok(MemoryEntry {
        id: row.get(0)?,
        kind: row.get(1)?,
        content: row.get(2)?,
        embedding: row.get::<_, Option<String>>(3)?.and_then(|s| serde_json::from_str(&s).ok()),
        importance: row.get(4)?,
        created_at: parse_dt(&row.get::<_, String>(5)?)?,
        accessed_at: parse_dt(&row.get::<_, String>(6)?)?,
        access_count: row.get(7)?,
    })
}

fn plan_row(row: &rusqlite::Row) -> rusqlite::Result<TaskPlan> {
    Ok(TaskPlan {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        status: row.get(3)?,
        steps: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or_default(),
        current_step: row.get::<_, i64>(5)? as usize,
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
        updated_at: parse_dt(&row.get::<_, String>(7)?)?,
        completed_at: row.get::<_, Option<String>>(8)?.and_then(|s| parse_dt(&s).ok()),
        result: row.get(9)?,
    })
}

fn reflection_row(row: &rusqlite::Row) -> rusqlite::Result<Reflection> {
    Ok(Reflection {
        id: row.get(0)?,
        plan_id: row.get(1)?,
        what_went_well: row.get(2)?,
        what_failed: row.get(3)?,
        lessons_learned: row.get(4)?,
        score: row.get(5)?,
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
    })
}

fn tool_row(row: &rusqlite::Row) -> rusqlite::Result<ToolEntry> {
    Ok(ToolEntry {
        name: row.get(0)?,
        description: row.get(1)?,
        schema: row.get(2)?,
        enabled: row.get(3)?,
        success_count: row.get(4)?,
        failure_count: row.get(5)?,
    })
}

fn prompt_row(row: &rusqlite::Row) -> rusqlite::Result<(String, f64, i64)> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

fn audit_row(row: &rusqlite::Row) -> rusqlite::Result<AuditEvent> {
    Ok(AuditEvent {
        id: row.get(0)?,
        event_type: row.get(1)?,
        details: serde_json::from_str(&row.get::<_, String>(2)?).unwrap_or(serde_json::Value::Null),
        created_at: parse_dt(&row.get::<_, String>(3)?)?,
    })
}

fn skill_row(row: &rusqlite::Row) -> rusqlite::Result<SkillDefinition> {
    Ok(SkillDefinition {
        id: row.get(0)?,
        name: row.get(1)?,
        description: row.get(2)?,
        tags: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
        params: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(serde_json::Value::Null),
        implementation: row.get(5)?,
        source: row.get(6)?,
        confidence: row.get(7)?,
        success_count: row.get(8)?,
        failure_count: row.get(9)?,
        created_at: parse_dt(&row.get::<_, String>(10)?)?,
        updated_at: parse_dt(&row.get::<_, String>(11)?)?,
    })
}

fn task_case_row(row: &rusqlite::Row) -> rusqlite::Result<TaskCase> {
    Ok(TaskCase {
        id: row.get(0)?,
        title: row.get(1)?,
        description: row.get(2)?,
        input_example: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or(serde_json::Value::Null),
        output_example: serde_json::from_str(&row.get::<_, String>(4)?).unwrap_or(serde_json::Value::Null),
        tags: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or_default(),
        source: row.get(6)?,
        created_at: parse_dt(&row.get::<_, String>(7)?)?,
    })
}

fn model_row(row: &rusqlite::Row) -> rusqlite::Result<ModelRecord> {
    Ok(ModelRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        source: row.get(2)?,
        model_type: row.get(3)?,
        format: row.get(4)?,
        path: row.get(5)?,
        url: row.get(6)?,
        metadata: serde_json::from_str(&row.get::<_, String>(7)?).unwrap_or(serde_json::Value::Null),
        created_at: parse_dt(&row.get::<_, String>(8)?)?,
    })
}

fn dataset_row(row: &rusqlite::Row) -> rusqlite::Result<DatasetRecord> {
    Ok(DatasetRecord {
        id: row.get(0)?,
        name: row.get(1)?,
        source: row.get(2)?,
        size: row.get::<_, i64>(3)? as usize,
        format: row.get(4)?,
        metadata: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(serde_json::Value::Null),
        created_at: parse_dt(&row.get::<_, String>(6)?)?,
    })
}

fn training_job_row(row: &rusqlite::Row) -> rusqlite::Result<TrainingJob> {
    Ok(TrainingJob {
        id: row.get(0)?,
        model_id: row.get(1)?,
        mode: row.get(2)?,
        dataset_ids: serde_json::from_str(&row.get::<_, String>(3)?).unwrap_or_default(),
        status: row.get(4)?,
        metrics: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(serde_json::Value::Null),
        artifact_path: row.get(6)?,
        created_at: parse_dt(&row.get::<_, String>(7)?)?,
        updated_at: parse_dt(&row.get::<_, String>(8)?)?,
    })
}
