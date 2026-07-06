//! The nervous system's observability tap: a real, append-only, queryable
//! record of every sandboxed invocation — which runtime ran, what it was
//! allowed to touch, how long it took, and whether it was let through,
//! denied by a capability check, or killed for exceeding its timeout. This
//! is what makes the sandbox's decisions inspectable by a person instead of
//! being an invisible internal detail; the GUI's Console surfaces it
//! directly (`get_nervous_system_audit` Tauri command).
use rusqlite::Connection;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

/// One audited event. `outcome` is one of `"allowed"`, `"capability_denied"`,
/// or `"timeout_killed"` — kept as a plain string (not an enum) so the
/// SQLite schema and the JSON the GUI reads stay the same shape.
pub struct AuditEvent<'a> {
    pub runtime: &'a str,
    pub outcome: &'a str,
    pub duration_ms: Option<u64>,
    pub detail: &'a str,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditRecord {
    pub runtime: String,
    pub outcome: String,
    pub duration_ms: Option<u64>,
    pub detail: String,
    pub logged_at: String,
}

static AUDIT_DB: OnceLock<Mutex<Connection>> = OnceLock::new();

fn audit_log_path() -> PathBuf {
    std::env::temp_dir().join(format!("brainbuilder_nervous_system_audit_{}.sqlite3", std::process::id()))
}

fn db() -> &'static Mutex<Connection> {
    AUDIT_DB.get_or_init(|| {
        let conn = Connection::open(audit_log_path())
            .unwrap_or_else(|_| Connection::open_in_memory().expect("in-memory audit db always opens"));
        conn.execute(
            "CREATE TABLE IF NOT EXISTS nervous_system_audit (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                runtime TEXT NOT NULL,
                outcome TEXT NOT NULL,
                duration_ms INTEGER,
                detail TEXT NOT NULL,
                logged_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            )",
            [],
        )
        .expect("create nervous_system_audit table");
        Mutex::new(conn)
    })
}

/// Records one audited event. Best-effort by design: a write failure here
/// (e.g. a poisoned lock after a panic elsewhere) must never fail the
/// sandboxed call it's describing.
pub fn record(event: AuditEvent) {
    let Ok(conn) = db().lock() else { return };
    let _ = conn.execute(
        "INSERT INTO nervous_system_audit (runtime, outcome, duration_ms, detail) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![event.runtime, event.outcome, event.duration_ms, event.detail],
    );
}

/// A small helper for the common "time the call, record allowed/denied"
/// pattern used by `Supervisor` and `PythonBridge`.
pub struct Timer(Instant);

impl Timer {
    pub fn start() -> Self {
        Self(Instant::now())
    }
    pub fn elapsed_ms(&self) -> u64 {
        self.0.elapsed().as_millis() as u64
    }
}

/// Most-recent audit events, newest first, for the GUI's Console panel.
pub fn recent(limit: i64) -> Vec<AuditRecord> {
    let Ok(conn) = db().lock() else { return Vec::new() };
    let mut stmt = match conn.prepare(
        "SELECT runtime, outcome, duration_ms, detail, logged_at
         FROM nervous_system_audit ORDER BY id DESC LIMIT ?1",
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = stmt.query_map([limit], |row| {
        Ok(AuditRecord {
            runtime: row.get(0)?,
            outcome: row.get(1)?,
            duration_ms: row.get(2)?,
            detail: row.get(3)?,
            logged_at: row.get(4)?,
        })
    });
    match rows {
        Ok(rows) => rows.filter_map(|r| r.ok()).collect(),
        Err(_) => Vec::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_and_reads_back_an_event() {
        record(AuditEvent {
            runtime: "test-runtime",
            outcome: "allowed",
            duration_ms: Some(12),
            detail: "unit test invocation",
        });
        let events = recent(50);
        assert!(events.iter().any(|e| e.runtime == "test-runtime" && e.detail == "unit test invocation"));
    }

    #[test]
    fn newest_event_comes_first() {
        record(AuditEvent { runtime: "order-test", outcome: "allowed", duration_ms: None, detail: "first" });
        record(AuditEvent { runtime: "order-test", outcome: "allowed", duration_ms: None, detail: "second" });
        let events = recent(50);
        let idx_first = events.iter().position(|e| e.detail == "first").unwrap();
        let idx_second = events.iter().position(|e| e.detail == "second").unwrap();
        assert!(idx_second < idx_first, "second recorded event should sort before first");
    }
}
