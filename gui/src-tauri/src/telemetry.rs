use std::sync::Arc;
use tokio::sync::Mutex;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TelemetryEvent {
    pub id: String,
    pub category: String,
    pub event: String,
    pub outcome: String,
    pub latency_ms: u64,
    pub payload: serde_json::Value,
    pub created_at: String,
}

#[derive(Clone)]
pub struct TelemetryStore {
    conn: Arc<Mutex<rusqlite::Connection>>,
}

impl TelemetryStore {
    pub fn new<P: AsRef<std::path::Path>>(path: P) -> Result<Self, String> {
        std::fs::create_dir_all(path.as_ref().parent().unwrap_or_else(|| std::path::Path::new("."))).ok();
        let conn = rusqlite::Connection::open(path).map_err(|e| e.to_string())?;
        let store = Self { conn: Arc::new(Mutex::new(conn)) };
        store.init_schema()?;
        Ok(store)
    }

    fn init_schema(&self) -> Result<(), String> {
        let c = self.conn.blocking_lock();
        c.execute(
            "CREATE TABLE IF NOT EXISTS telemetry (id TEXT PRIMARY KEY, category TEXT, event TEXT, outcome TEXT, latency_ms INTEGER, payload TEXT, created_at TEXT)",
            [],
        ).map_err(|e| e.to_string())?;
        c.execute(
            "CREATE INDEX IF NOT EXISTS idx_telemetry_category ON telemetry(category)",
            [],
        ).map_err(|e| e.to_string())?;
        Ok(())
    }

    pub fn record(&self, evt: TelemetryEvent) -> Result<(), String> {
        let c = self.conn.blocking_lock();
        c.execute(
            "INSERT INTO telemetry (id, category, event, outcome, latency_ms, payload, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            &[
                &evt.id,
                &evt.category,
                &evt.event,
                &evt.outcome,
                &evt.latency_ms.to_string(),
                &evt.payload.to_string(),
                &evt.created_at,
            ],
        ).map_err(|e| e.to_string())?;
        tracing::info!(category=%evt.category, event=%evt.event, outcome=%evt.outcome, "telemetry recorded");
        Ok(())
    }

    pub fn query(&self, category: &str, limit: usize) -> Result<Vec<TelemetryEvent>, String> {
        let c = self.conn.blocking_lock();
        let mut stmt = c.prepare("SELECT id, category, event, outcome, latency_ms, payload, created_at FROM telemetry WHERE category = ?1 ORDER BY created_at DESC LIMIT ?2").map_err(|e| e.to_string())?;
        let rows = stmt.query_map([category, &limit.to_string()], |row| {
            Ok(TelemetryEvent {
                id: row.get(0)?,
                category: row.get(1)?,
                event: row.get(2)?,
                outcome: row.get(3)?,
                latency_ms: row.get(4)?,
                payload: serde_json::from_str(&row.get::<_, String>(5)?).unwrap_or(serde_json::Value::Null),
                created_at: row.get(6)?,
            })
        }).map_err(|e| e.to_string())?;
        let mut out = Vec::new();
        for row in rows {
            out.push(row.map_err(|e| e.to_string())?);
        }
        Ok(out)
    }

    pub fn success_rate(&self, event: &str, outcome: &str) -> Result<f64, String> {
        let c = self.conn.blocking_lock();
        let total: i64 = c.query_row("SELECT COUNT(*) FROM telemetry WHERE event = ?1", [event], |row| row.get(0)).unwrap_or(0);
        let successes: i64 = if outcome == "*" {
            total
        } else {
            c.query_row("SELECT COUNT(*) FROM telemetry WHERE event = ?1 AND outcome = ?2", [event, outcome], |row| row.get(0)).unwrap_or(0)
        };
        if total == 0 { Ok(1.0) } else { Ok(successes as f64 / total as f64) }
    }
}
