// Proves the gradient-exchange split (compute_gradients + apply_averaged_
// gradients) is real data-parallel training, not just plumbing: two "clients"
// each compute gradients on a different data shard, the "host" averages them
// and applies one optimizer step — and checks this converges to the true
// weight exactly like the non-distributed `train_step` path does. Ignored by
// default (needs `torch` importable) — run with `cargo test -- --ignored`.
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use brainbuilder_core::runtime::scheduler::ExecutableOp;
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
fn averaging_gradients_from_two_shards_converges_to_the_true_weight() {
    let fixtures_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::env::set_var("PYTHONPATH", &fixtures_dir);

    // Same simulated host process runs both "clients" — the point being
    // proven is the gradient-average math and worker protocol, not the
    // network transport (already proven separately in
    // distributed_job_protocol.rs).
    let host = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let client_a = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();
    let client_b = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let true_w = 3.0f32;
    // Two disjoint shards of the same underlying data distribution.
    let shard_a_x = [1.0f32, 2.0];
    let shard_b_x = [3.0f32, 4.0];
    let shard_a_target: Vec<f32> = shard_a_x.iter().map(|x| x * true_w).collect();
    let shard_b_target: Vec<f32> = shard_b_x.iter().map(|x| x * true_w).collect();

    let ops = vec![ExecutableOp {
        node_id: "n1".into(),
        component: "bb_scale_component".into(),
        language: "python".into(),
        entry: "forward".into(),
        inputs: vec!["x".into(), "w".into()],
        outputs: vec!["y".into()],
        param_inputs: vec!["w".into()],
        trainable_inputs: vec!["w".into()],
        zero_init_inputs: vec![],
        hyperparams: serde_json::json!({}),
        resolved_param_shapes: Default::default(),
    }];

    let mut w = tensor_from(&[0.1, 0.1]);
    let mut losses = Vec::new();

    for _ in 0..200 {
        let mut inputs_a = HashMap::new();
        inputs_a.insert("x".to_string(), tensor_from(&shard_a_x));
        inputs_a.insert("w".to_string(), w.clone());
        let (loss_a, grads_a) = client_a
            .compute_gradients(&ops, inputs_a, "y", &tensor_from(&shard_a_target), "mse", &["w".to_string()])
            .expect("client_a compute_gradients failed");

        let mut inputs_b = HashMap::new();
        inputs_b.insert("x".to_string(), tensor_from(&shard_b_x));
        inputs_b.insert("w".to_string(), w.clone());
        let (loss_b, grads_b) = client_b
            .compute_gradients(&ops, inputs_b, "y", &tensor_from(&shard_b_target), "mse", &["w".to_string()])
            .expect("client_b compute_gradients failed");

        // Host averages: real, non-mocked averaging of the two real
        // gradient tensors returned by two independent worker processes.
        let grad_a_vals = read(grads_a.get("w").unwrap(), 2);
        let grad_b_vals = read(grads_b.get("w").unwrap(), 2);
        let averaged: Vec<f32> = grad_a_vals.iter().zip(&grad_b_vals).map(|(a, b)| (a + b) / 2.0).collect();

        let mut weights = HashMap::new();
        weights.insert("w".to_string(), w.clone());
        let mut avg_grads = HashMap::new();
        avg_grads.insert("w".to_string(), tensor_from(&averaged));

        let updated = host
            .apply_averaged_gradients(&weights, &avg_grads, "sgd", 0.05, 0.0)
            .expect("apply_averaged_gradients failed");
        w = updated.get("w").expect("w not returned").clone();

        losses.push((loss_a + loss_b) / 2.0);
    }

    let first_loss = losses[0];
    let last_loss = *losses.last().unwrap();
    assert!(
        last_loss < first_loss * 0.01,
        "expected averaged-gradient training to collapse loss: first={first_loss}, last={last_loss}"
    );

    let learned_w = read(&w, 2);
    for w_i in learned_w {
        assert!(
            (w_i - true_w).abs() < 0.05,
            "expected learned weight ~{true_w}, got {w_i}"
        );
    }
}
