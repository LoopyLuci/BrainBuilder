// Exercises the real DLPack<->PyCapsule<->torch.Tensor bridge in
// `core/src/interop/python.rs` against a live Python 3.11 + PyTorch install.
// Ignored by default (needs `torch` importable) — run with
// `cargo test -- --ignored`.
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use std::sync::Arc;

#[test]
#[ignore]
fn round_trips_a_tensor_through_torch_identity() {
    // Make the fixture module importable.
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::env::set_var("PYTHONPATH", &fixtures_dir);

    let device = CpuDevice;
    let tensor = device.alloc(&[2, 2], DataType::Float32);

    // Write known values into the raw CPU buffer.
    unsafe {
        let data_ptr = (*tensor.0).dl_tensor.data as *mut f32;
        for (i, v) in [1.0f32, 2.0, 3.0, 4.0].iter().enumerate() {
            *data_ptr.add(i) = *v;
        }
    }

    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let result = bridge
        .call_component("bb_echo_component", "identity", vec![tensor], &serde_json::json!({}))
        .expect("round trip through torch identity failed");

    unsafe {
        let data_ptr = (*result.0).dl_tensor.data as *const f32;
        let values: Vec<f32> = (0..4).map(|i| *data_ptr.add(i)).collect();
        assert_eq!(values, vec![1.0, 2.0, 3.0, 4.0]);
    }
}
