// Proves the real bug fix: dropout used to always drop units at rate `p`,
// even during a `predict` forward pass — meaning every real prediction from
// a graph containing a dropout node was silently non-deterministic and
// randomly degraded. `dropout.py` now accepts a `training` kwarg (default
// True, so every existing train_step/compute_gradients call is unaffected),
// and `ExecutionPlan::forward` (the inference-only path behind Predict,
// batch predict, the local serve endpoint, and feature importance) now
// explicitly passes `training=false`. Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, PortInfo};
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::orchestrator::Orchestrator;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use std::sync::Arc;

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

/// Directly exercises `dropout.py`'s new `training` kwarg (bypassing the
/// Rust wiring): default (unset, `training=True`) still drops real units at
/// a high rate, exactly like before this fix — every train_step call must
/// keep behaving identically.
#[test]
#[ignore]
fn dropout_still_drops_units_by_default_matching_prior_training_behavior() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let ones = vec![1.0f32; 1000];
    let input = tensor_shaped(&[1000], &ones);
    let output = bridge
        .call_component("dropout", "forward", vec![input], &serde_json::json!({"p": 0.9}))
        .expect("dropout call failed");

    let values = read(&output, 1000);
    let zero_count = values.iter().filter(|&&v| v == 0.0).count();
    // p=0.9 on 1000 samples: ~900 zeros expected. Loose bound to avoid
    // flakiness while still clearly proving real stochastic dropping.
    assert!(
        zero_count > 700,
        "expected the large majority of 1000 units to be dropped at p=0.9, got only {zero_count} zeros"
    );
}

/// Directly exercises `training=false`: the component itself must become a
/// pure pass-through, ignoring `p` entirely, regardless of the Rust wiring.
#[test]
#[ignore]
fn dropout_is_a_pass_through_in_eval_mode_regardless_of_p() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let input = tensor_shaped(&[4], &[1.0, 2.0, 3.0, 4.0]);
    let output = bridge
        .call_component("dropout", "forward", vec![input], &serde_json::json!({"p": 0.9, "training": false}))
        .expect("dropout call failed");

    assert_eq!(read(&output, 4), vec![1.0, 2.0, 3.0, 4.0]);
}

/// The real regression test: exercises the actual Rust wiring
/// (`Orchestrator::predict` -> `ExecutionPlan::forward`) with a graph whose
/// only node is a high-p dropout, no checkpoint needed (dropout has no
/// learnable parameters). Before this fix, two predictions on the same
/// input would almost certainly differ (each one randomly zeroing ~90% of a
/// 200-element vector). After the fix, both calls must be bit-identical to
/// the input — proving `training=false` actually reaches the component
/// through the full inference path, not just when called directly.
#[test]
#[ignore]
fn real_predict_pass_is_deterministic_through_a_high_p_dropout_node() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "dropout-eval-mode-test".to_string(),
        name: "dropout-eval-mode-test".to_string(),
        nodes: vec![BBIRNode {
            id: "n1".to_string(),
            component: "dropout".to_string(),
            label: None,
            hyperparams: serde_json::json!({"p": 0.9}),
            ports: PortInfo {
                input_ports: vec!["input".to_string()],
                output_ports: vec!["output".to_string()],
            },
            position: None,
        }],
        edges: Vec::<BBIREdge>::new(),
        training: None,
    };

    let values: Vec<f32> = (0..200).map(|i| 1.0 + i as f32 * 0.01).collect();
    let csv_dir = std::env::temp_dir().join("bb_dropout_eval_mode");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    let mut csv = String::from("x\n");
    for v in &values {
        csv.push_str(&format!("{v}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let rt = tokio::runtime::Runtime::new().unwrap();
    let batch = rt
        .block_on(brainbuilder_core::data::source::load_batch(&csv_path.to_string_lossy(), 200))
        .expect("failed to load a batch for prediction");

    let first = orchestrator.predict(graph.clone(), batch.clone()).expect("first predict failed");
    let second = orchestrator.predict(graph, batch).expect("second predict failed");

    let first_values = read(&first[0], 200);
    let second_values = read(&second[0], 200);

    assert_eq!(first_values, values, "eval-mode dropout must be an exact pass-through of the input");
    assert_eq!(second_values, values, "eval-mode dropout must be an exact pass-through of the input");
}
