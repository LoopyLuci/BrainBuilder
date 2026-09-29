// Proves the shipped `components/python/*.py` files are real, correct
// implementations — not just importable stubs. `linear.edn`/`relu.edn`
// pointed at `linear.py`/`relu.py` since the original blueprint, but no such
// files ever existed anywhere in the repo; training any real graph would
// have failed with `ModuleNotFoundError` the moment a user tried it. Ignored
// by default (needs `torch` importable) — run with `cargo test -- --ignored`.
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use std::sync::Arc;

fn tensor_from(values: &[f32]) -> brainbuilder_core::Tensor {
    tensor_shaped(&[values.len()], values)
}

fn tensor_shaped(shape: &[usize], values: &[f32]) -> brainbuilder_core::Tensor {
    let device = CpuDevice;
    let t = device.alloc(shape, DataType::Float32);
    unsafe {
        let dst = (*t.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
    }
    t
}

fn read(tensor: &brainbuilder_core::Tensor, len: usize) -> Vec<f32> {
    unsafe { std::slice::from_raw_parts((*tensor.0).dl_tensor.data as *const f32, len).to_vec() }
}

fn set_components_python_path() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components/python");
    std::env::set_var("PYTHONPATH", dir);
}

#[test]
#[ignore]
fn linear_component_computes_a_real_linear_layer() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    // 1x2 input, 1x2 weight (out_features=1, in_features=2): output should be
    // the dot product, y = 1*3 + 2*4 = 11.
    let input = tensor_from(&[1.0, 2.0]);
    let weight = tensor_from(&[3.0, 4.0]);

    let output = bridge
        .call_component("linear", "forward", vec![input, weight], &serde_json::json!({}))
        .expect("linear component call failed");

    assert_eq!(read(&output, 1), vec![11.0]);
}

#[test]
#[ignore]
fn relu_component_zeroes_negatives() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let input = tensor_from(&[-1.0, 0.0, 2.5, -3.0]);
    let output = bridge
        .call_component("relu", "relu", vec![input], &serde_json::json!({}))
        .expect("relu component call failed");

    assert_eq!(read(&output, 4), vec![0.0, 0.0, 2.5, 0.0]);
}

#[test]
#[ignore]
fn conv2d_component_computes_a_real_convolution() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    // 1x1x3x3 input, 1x1x2x2 all-ones kernel: each output cell is the sum of
    // its 2x2 window (valid conv, no padding) -> a real, checkable result.
    #[rustfmt::skip]
    let input = tensor_shaped(&[1, 1, 3, 3], &[
        1.0, 2.0, 3.0,
        4.0, 5.0, 6.0,
        7.0, 8.0, 9.0,
    ]);
    let weight = tensor_shaped(&[1, 1, 2, 2], &[1.0, 1.0, 1.0, 1.0]);

    let output = bridge
        .call_component("conv2d", "forward", vec![input, weight], &serde_json::json!({}))
        .expect("conv2d component call failed");

    // windows: (1+2+4+5)=12, (2+3+5+6)=16, (4+5+7+8)=24, (5+6+8+9)=28
    assert_eq!(read(&output, 4), vec![12.0, 16.0, 24.0, 28.0]);
}

#[test]
#[ignore]
fn adam_component_computes_a_real_first_step_update() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let params = tensor_from(&[1.0, 2.0]);
    let grads = tensor_from(&[0.1, -0.2]);

    let output = bridge
        .call_component("adam", "step", vec![params, grads], &serde_json::json!({}))
        .expect("adam component call failed");

    // Step-1 Adam collapses to `p - lr * sign(g)` (m_hat=g, v_hat=g^2, so
    // g/(|g|+eps) ~= sign(g)); default lr=0.001.
    let result = read(&output, 2);
    assert!((result[0] - (1.0 - 0.001)).abs() < 1e-3);
    assert!((result[1] - (2.0 + 0.001)).abs() < 1e-3);
}
