// Proves the sandbox-hardening pass is real: a genuinely stuck worker call
// times out (rather than hanging the caller forever) and the *next* call on
// the same `PythonBridge` still succeeds (transparent respawn). Ignored by
// default (needs torch) — run with `cargo test -- --ignored`.
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use std::sync::Arc;
use std::time::Duration;

#[test]
#[ignore]
fn stuck_worker_call_times_out_and_the_bridge_recovers() {
    let bridge = PythonBridge::new(Arc::new(SharedArena::new()))
        .unwrap()
        .with_timeout_for_testing(Duration::from_secs(3));

    // Sleeps far longer than the timeout -> must error, not hang. The gap is
    // wide (3s vs 10s) since running alongside other tests in the full suite
    // adds real scheduling/startup jitter that a tight margin flakes on.
    let result = bridge.sleep_for_testing(10.0);
    assert!(result.is_err(), "expected the stuck call to time out");

    // The worker was killed and respawned inside `request()` — a fresh call
    // must still work, proving the bridge recovered rather than being
    // permanently wedged.
    let recovered = bridge.random_tensor(&[3]);
    assert!(recovered.is_ok(), "bridge should have respawned and recovered");
}
