// Proves DeviceProfile::capture finds this machine's *real* GPU (not a
// mocked/empty list) when the wgpu feature is on. Ignored by default (needs
// a real GPU-capable adapter) — run with
// `cargo test --features wgpu,pollster -- --ignored`.
#![cfg(feature = "wgpu")]
use brainbuilder_core::cluster::DeviceProfile;

#[test]
#[ignore]
fn captures_a_real_gpu_on_this_machine() {
    let profile = DeviceProfile::capture("test-desktop");
    eprintln!("captured profile: {profile:?}");
    assert!(!profile.gpus.is_empty(), "expected at least one real GPU adapter to be enumerated");
    assert!(profile.cpu_cores >= 1);
    assert!(profile.ram_total_bytes.unwrap_or(0) > 0);
}
