// Proves checkpoint save/load is real: trains a weight to a known value,
// saves it, loads it back into a fresh HashMap, and checks the values match
// exactly. Ignored by default (needs `torch` importable).
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use std::collections::HashMap;
use std::sync::Arc;

fn tensor_from(values: &[f32]) -> brainbuilder_core::Tensor {
    let device = CpuDevice;
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
fn checkpoint_round_trips_weight_values() {
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let mut weights: HashMap<String, brainbuilder_core::Tensor> = HashMap::new();
    weights.insert("weight".to_string(), tensor_from(&[1.5, -2.25, 3.0]));
    weights.insert("bias".to_string(), tensor_from(&[0.1]));

    let path = std::env::temp_dir().join("bb_checkpoint_test.pt");
    bridge.save_state_dict(&weights, &path).expect("save failed");

    let loaded = bridge.load_state_dict(&path).expect("load failed");
    assert_eq!(read(loaded.get("weight").unwrap(), 3), vec![1.5, -2.25, 3.0]);
    assert_eq!(read(loaded.get("bias").unwrap(), 1), vec![0.1]);

    std::fs::remove_file(&path).ok();
}
