// Real append-only persistence via SQLite (`rusqlite`, bundled — no system
// libsqlite3 needed to build). One row per logged record; the table is
// genuinely append-only (`INSERT` only, no `UPDATE`/`DELETE` anywhere here).
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// Record the exact data-sample -> weight-update lineage.
pub struct ProvenanceRecord {
    pub graph_version: String,
    pub node_id: String,
    pub sample_id: u64,
    pub step: usize,
}

pub struct ProvenanceStore {
    conn: Mutex<Connection>,
}

impl ProvenanceStore {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to open provenance store: {e}")))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provenance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                graph_version TEXT NOT NULL,
                node_id TEXT NOT NULL,
                sample_id INTEGER NOT NULL,
                step INTEGER NOT NULL,
                logged_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
            [],
        )
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to create provenance table: {e}")))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// In-memory store — for tests, or short-lived processes that don't need
    /// lineage to survive a restart.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to open provenance store: {e}")))?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS provenance (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                graph_version TEXT NOT NULL,
                node_id TEXT NOT NULL,
                sample_id INTEGER NOT NULL,
                step INTEGER NOT NULL,
                logged_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
            [],
        )
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to create provenance table: {e}")))?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn log(&self, record: ProvenanceRecord) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| BrainBuilderError::ConfigError("provenance store lock poisoned".into()))?;
        conn.execute(
            "INSERT INTO provenance (graph_version, node_id, sample_id, step) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![record.graph_version, record.node_id, record.sample_id, record.step],
        )
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to write provenance record: {e}")))?;
        Ok(())
    }

    pub fn count(&self) -> Result<u64> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| BrainBuilderError::ConfigError("provenance store lock poisoned".into()))?;
        conn.query_row("SELECT COUNT(*) FROM provenance", [], |row| row.get(0))
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to count provenance records: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logs_and_counts_records() {
        let store = ProvenanceStore::in_memory().unwrap();
        for step in 0..3 {
            store
                .log(ProvenanceRecord {
                    graph_version: "g1".into(),
                    node_id: "n1".into(),
                    sample_id: step as u64,
                    step,
                })
                .unwrap();
        }
        assert_eq!(store.count().unwrap(), 3);
    }
}
