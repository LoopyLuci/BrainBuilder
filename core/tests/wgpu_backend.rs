// Proves the wgpu backend runs real compute on a real GPU adapter (Vulkan/
// DX12/Metal, whatever the OS gives us) — not just that it compiles. Ignored
// by default (needs a real GPU-capable adapter) — run with
// `cargo test --features wgpu,pollster -- --ignored`.
#![cfg(feature = "wgpu")]
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::runtime::device::Device;
use brainbuilder_core::runtime::wgpu_backend::WgpuDevice;

fn tensor_from(values: &[f32]) -> brainbuilder_core::Tensor {
    let device = brainbuilder_core::runtime::cpu_backend::CpuDevice;
    let t = device.alloc(&[values.len()], DataType::Float32);
    unsafe {
        let dst = (*t.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
    }
    t
}

fn read(tensor: &brainbuilder_core::Tensor, len: usize) -> Vec<f32> {
    unsafe { std::slice::from_raw_parts((*tensor.0).dl_tensor.data as *const f32, len).to_vec() }
}

#[test]
#[ignore]
fn relu_runs_on_a_real_gpu_adapter() {
    let device = WgpuDevice::new().expect("no real GPU adapter available");
    eprintln!("running on adapter: {}", device.adapter_name());

    let input = tensor_from(&[-2.0, -0.5, 0.0, 1.5, 3.0]);
    let output = device.exec("relu", &[&input]).expect("relu compute shader failed");

    assert_eq!(read(&output, 5), vec![0.0, 0.0, 0.0, 1.5, 3.0]);
}

#[test]
#[ignore]
fn identity_runs_on_a_real_gpu_adapter() {
    let device = WgpuDevice::new().expect("no real GPU adapter available");
    let input = tensor_from(&[1.0, 2.0, 3.0]);
    let output = device.exec("identity", &[&input]).expect("identity compute shader failed");
    assert_eq!(read(&output, 3), vec![1.0, 2.0, 3.0]);
}
