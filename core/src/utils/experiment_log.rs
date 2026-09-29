// Real append-only persistence via SQLite (mirrors `provenance.rs`'s
// pattern) for a lightweight training-run history: "did lr=0.01 or lr=0.001
// train better on this graph?" without the user needing to remember or
// re-derive past results by hand. One row per finished `execute_graph` call;
// the table is genuinely append-only (INSERT only, no UPDATE/DELETE).
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use rusqlite::Connection;
use std::path::Path;
use std::sync::Mutex;

/// What a caller supplies when logging a finished run — no `id`/`logged_at`,
/// since those are assigned by the database.
pub struct NewExperiment {
    pub graph_id: String,
    pub graph_name: String,
    /// A legible architecture signature — each node's component name, in
    /// graph order, joined with " → " (e.g. "linear → relu → linear"). Lets
    /// the History table compare runs across genuinely different graph
    /// structures, not just different hyperparameters on the same one.
    pub architecture: String,
    pub loss_fn: String,
    pub optimizer: String,
    pub lr: f64,
    pub batch_size: i64,
    pub epochs: i64,
    /// The loss of the first and last metric points observed during the run.
    /// `None` if the run produced no metric points at all (e.g. an
    /// immediately-failing trainer_type this log doesn't otherwise reject).
    pub first_loss: Option<f64>,
    pub last_loss: Option<f64>,
}

/// A full row as read back — what the GUI's history table renders.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExperimentRecord {
    pub graph_id: String,
    pub graph_name: String,
    pub architecture: String,
    pub loss_fn: String,
    pub optimizer: String,
    pub lr: f64,
    pub batch_size: i64,
    pub epochs: i64,
    pub first_loss: Option<f64>,
    pub last_loss: Option<f64>,
    pub logged_at: String,
}

pub struct ExperimentLog {
    conn: Mutex<Connection>,
}

const CREATE_TABLE_SQL: &str = "CREATE TABLE IF NOT EXISTS experiments (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    graph_id TEXT NOT NULL,
    graph_name TEXT NOT NULL,
    loss_fn TEXT NOT NULL,
    optimizer TEXT NOT NULL,
    lr REAL NOT NULL,
    batch_size INTEGER NOT NULL,
    epochs INTEGER NOT NULL,
    first_loss REAL,
    last_loss REAL,
    logged_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
)";

/// Added after `experiments` already shipped without it — a plain `ALTER
/// TABLE` on a fresh (or already-migrated) database, tolerating the
/// "duplicate column" error a repeat run produces instead of requiring
/// SQLite's newer `ADD COLUMN IF NOT EXISTS` syntax.
fn migrate_architecture_column(conn: &Connection) -> Result<()> {
    match conn.execute("ALTER TABLE experiments ADD COLUMN architecture TEXT NOT NULL DEFAULT ''", []) {
        Ok(_) => Ok(()),
        Err(rusqlite::Error::SqliteFailure(_, Some(msg))) if msg.contains("duplicate column name") => Ok(()),
        Err(e) => Err(BrainBuilderError::ConfigError(format!("failed to add architecture column: {e}"))),
    }
}

impl ExperimentLog {
    pub fn open(path: &Path) -> Result<Self> {
        let conn = Connection::open(path)
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to open experiment log: {e}")))?;
        conn.execute(CREATE_TABLE_SQL, [])
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to create experiments table: {e}")))?;
        migrate_architecture_column(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    /// In-memory store — for tests, or short-lived processes that don't need
    /// history to survive a restart.
    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to open experiment log: {e}")))?;
        conn.execute(CREATE_TABLE_SQL, [])
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to create experiments table: {e}")))?;
        migrate_architecture_column(&conn)?;
        Ok(Self { conn: Mutex::new(conn) })
    }

    pub fn log(&self, record: NewExperiment) -> Result<()> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| BrainBuilderError::ConfigError("experiment log lock poisoned".into()))?;
        conn.execute(
            "INSERT INTO experiments (graph_id, graph_name, architecture, loss_fn, optimizer, lr, batch_size, epochs, first_loss, last_loss)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![
                record.graph_id,
                record.graph_name,
                record.architecture,
                record.loss_fn,
                record.optimizer,
                record.lr,
                record.batch_size,
                record.epochs,
                record.first_loss,
                record.last_loss,
            ],
        )
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to write experiment record: {e}")))?;
        Ok(())
    }

    /// The most recent `limit` runs, newest first.
    pub fn recent(&self, limit: usize) -> Result<Vec<ExperimentRecord>> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| BrainBuilderError::ConfigError("experiment log lock poisoned".into()))?;
        let mut stmt = conn
            .prepare(
                "SELECT graph_id, graph_name, architecture, loss_fn, optimizer, lr, batch_size, epochs, first_loss, last_loss, logged_at
                 FROM experiments ORDER BY id DESC LIMIT ?1",
            )
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to prepare experiment query: {e}")))?;
        let rows = stmt
            .query_map(rusqlite::params![limit as i64], |row| {
                Ok(ExperimentRecord {
                    graph_id: row.get(0)?,
                    graph_name: row.get(1)?,
                    architecture: row.get(2)?,
                    loss_fn: row.get(3)?,
                    optimizer: row.get(4)?,
                    lr: row.get(5)?,
                    batch_size: row.get(6)?,
                    epochs: row.get(7)?,
                    first_loss: row.get(8)?,
                    last_loss: row.get(9)?,
                    logged_at: row.get(10)?,
                })
            })
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to read experiment records: {e}")))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|e| BrainBuilderError::ConfigError(format!("failed to read experiment records: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(graph_id: &str, lr: f64, first: f64, last: f64) -> NewExperiment {
        NewExperiment {
            graph_id: graph_id.to_string(),
            graph_name: "test-graph".to_string(),
            architecture: "linear".to_string(),
            loss_fn: "mse".to_string(),
            optimizer: "sgd".to_string(),
            lr,
            batch_size: 32,
            epochs: 10,
            first_loss: Some(first),
            last_loss: Some(last),
        }
    }

    #[test]
    fn logs_and_reads_back_records_newest_first() {
        let log = ExperimentLog::in_memory().unwrap();
        log.log(sample("g1", 0.01, 1.0, 0.5)).unwrap();
        log.log(sample("g2", 0.001, 1.0, 0.9)).unwrap();

        let recent = log.recent(10).unwrap();
        assert_eq!(recent.len(), 2);
        // Newest (g2, logged second) comes first.
        assert_eq!(recent[0].graph_id, "g2");
        assert_eq!(recent[0].lr, 0.001);
        assert_eq!(recent[0].first_loss, Some(1.0));
        assert_eq!(recent[0].last_loss, Some(0.9));
        assert!(!recent[0].logged_at.is_empty());
        assert_eq!(recent[1].graph_id, "g1");
    }

    #[test]
    fn recent_respects_the_limit() {
        let log = ExperimentLog::in_memory().unwrap();
        for i in 0..5 {
            log.log(sample(&format!("g{i}"), 0.01, 1.0, 0.5)).unwrap();
        }
        assert_eq!(log.recent(2).unwrap().len(), 2);
        assert_eq!(log.recent(100).unwrap().len(), 5);
    }

    #[test]
    fn missing_metrics_are_stored_as_none() {
        let log = ExperimentLog::in_memory().unwrap();
        log.log(NewExperiment {
            graph_id: "g1".to_string(),
            graph_name: "test".to_string(),
            architecture: "linear".to_string(),
            loss_fn: "mse".to_string(),
            optimizer: "sgd".to_string(),
            lr: 0.01,
            batch_size: 8,
            epochs: 1,
            first_loss: None,
            last_loss: None,
        })
        .unwrap();
        let recent = log.recent(1).unwrap();
        assert_eq!(recent[0].first_loss, None);
        assert_eq!(recent[0].last_loss, None);
    }

    #[test]
    fn architecture_signature_round_trips() {
        let log = ExperimentLog::in_memory().unwrap();
        let mut record = sample("g1", 0.01, 1.0, 0.5);
        record.architecture = "linear → relu → linear".to_string();
        log.log(record).unwrap();

        let recent = log.recent(1).unwrap();
        assert_eq!(recent[0].architecture, "linear → relu → linear");
    }

    #[test]
    fn opening_a_pre_architecture_column_database_migrates_cleanly() {
        // Simulates a real user's existing experiments.sqlite3 from before the
        // architecture column existed: create the table with the *old* schema
        // by hand, then open it through `ExperimentLog::open` and confirm it
        // still works — no crash, no data loss, new column usable.
        let path = std::env::temp_dir().join(format!("bb_experiment_log_migration_{}.sqlite3", uuid::Uuid::new_v4()));
        {
            let conn = Connection::open(&path).unwrap();
            conn.execute(
                "CREATE TABLE experiments (
                    id INTEGER PRIMARY KEY AUTOINCREMENT,
                    graph_id TEXT NOT NULL,
                    graph_name TEXT NOT NULL,
                    loss_fn TEXT NOT NULL,
                    optimizer TEXT NOT NULL,
                    lr REAL NOT NULL,
                    batch_size INTEGER NOT NULL,
                    epochs INTEGER NOT NULL,
                    first_loss REAL,
                    last_loss REAL,
                    logged_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
                )",
                [],
            )
            .unwrap();
            conn.execute(
                "INSERT INTO experiments (graph_id, graph_name, loss_fn, optimizer, lr, batch_size, epochs)
                 VALUES ('old-g', 'old-graph', 'mse', 'sgd', 0.01, 16, 5)",
                [],
            )
            .unwrap();
        }

        let log = ExperimentLog::open(&path).unwrap();
        // The pre-existing row survives, with an empty (not missing/erroring)
        // architecture value.
        let recent = log.recent(10).unwrap();
        assert_eq!(recent.len(), 1);
        assert_eq!(recent[0].graph_id, "old-g");
        assert_eq!(recent[0].architecture, "");

        // And logging fresh rows with real architecture data works too.
        log.log(sample("new-g", 0.01, 1.0, 0.5)).unwrap();
        assert_eq!(log.recent(10).unwrap().len(), 2);

        std::fs::remove_file(&path).ok();
    }
}
