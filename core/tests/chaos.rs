// Real fault-injection ("chaos") tests for the crate's hardened paths, plus
// a couple of fast regression tests for bugs fixed in this session. Split
// out from `python_worker_hardening.rs` (which covers the timeout/respawn
// path) since these specifically externally-kill the real worker process
// rather than making it hang.
use brainbuilder_core::data::source::load_batch;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use std::sync::Arc;

/// Regression test for `load_batch` (`core/src/data/source.rs`): a
/// header-only CSV (zero data rows) used to index `batches[0]` directly and
/// panic. It now uses `batches.first()` and returns a clear
/// `BrainBuilderError` instead.
#[tokio::test]
async fn load_batch_on_a_header_only_csv_errors_instead_of_panicking() {
    let path = std::env::temp_dir().join(format!("bb_chaos_empty_{}.csv", uuid::Uuid::new_v4()));
    std::fs::write(&path, "col_a,col_b\n").expect("write fixture csv");

    let result = load_batch(&path.to_string_lossy(), 10).await;
    assert!(result.is_err(), "an empty (header-only) dataset must error, not panic, in load_batch");
    let msg = result.err().unwrap().to_string();
    assert!(
        msg.contains("no rows") || msg.to_lowercase().contains("no rows"),
        "expected a clear no-rows error, got: {msg}"
    );

    std::fs::remove_file(&path).ok();
}

/// Real chaos test: spawns a genuine `PythonBridge` (a real `_bb_worker.py`
/// subprocess), externally kills the worker process mid-lifecycle (not a
/// simulated failure — a real `taskkill`/`SIGKILL`), and asserts the bridge
/// auto-respawns so the *next* call still succeeds. Complements
/// `python_worker_hardening.rs`'s timeout/respawn coverage, which exercises
/// the hang path instead of a hard crash.
///
/// Ignored by default (needs python + torch on PATH) — run explicitly with
/// `cargo test -- --ignored`, matching this crate's existing convention for
/// tests needing a real interpreter/toolchain (see
/// `python_worker_hardening.rs`, `interop_smoke.rs`).
#[test]
#[ignore]
fn killing_the_worker_process_mid_flight_triggers_a_respawn_and_recovery() {
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).expect("bridge should spawn a real worker");

    // Prove the worker is alive and usable before the chaos injection.
    bridge.random_tensor(&[2, 2]).expect("initial call should succeed against a healthy worker");

    // Real fault injection: kill the worker process out from under the
    // bridge, simulating a crash (OOM-killed component code, a segfault in
    // a native torch op, etc.) rather than a graceful failure.
    bridge.kill_worker_for_testing().expect("killing the worker process should succeed");

    // The in-flight call after a kill should surface an error (the write or
    // the read side will observe the dead process) rather than hang forever.
    let during_crash = bridge.random_tensor(&[2, 2]);
    assert!(during_crash.is_err(), "a call against a just-killed worker should error, not silently succeed");

    // The critical assertion: the bridge must have respawned a fresh worker
    // as part of handling that failure, so the *next* call succeeds.
    let recovered = bridge.random_tensor(&[2, 2]);
    assert!(recovered.is_ok(), "bridge should have respawned and the next call should succeed");
}
