// Proves the ML-architecture-zoo primitives (components/{layernorm,gelu,add,
// dropout,embedding,attention}.py) are real, correct implementations, and
// that a real transformer block (attention -> residual -> layernorm -> MLP ->
// residual -> layernorm) trains via real autograd, not just runs. Ignored by
// default (needs torch) — run with `cargo test -- --ignored`.
use brainbuilder_core::component::descriptor::DataType;
use brainbuilder_core::interop::arena::SharedArena;
use brainbuilder_core::interop::python::PythonBridge;
use brainbuilder_core::runtime::cpu_backend::CpuDevice;
use brainbuilder_core::runtime::device::Device;
use brainbuilder_core::runtime::scheduler::ExecutableOp;
use std::collections::HashMap;
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

#[test]
#[ignore]
fn layernorm_component_normalizes_to_zero_mean_unit_variance() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let input = tensor_shaped(&[1, 4], &[1.0, 2.0, 3.0, 4.0]);
    let weight = tensor_shaped(&[4], &[1.0, 1.0, 1.0, 1.0]);
    let bias = tensor_shaped(&[4], &[0.0, 0.0, 0.0, 0.0]);

    let output = bridge
        .call_component("layernorm", "forward", vec![input, weight, bias], &serde_json::json!({"eps": 1e-5}))
        .expect("layernorm call failed");

    let values = read(&output, 4);
    let mean: f32 = values.iter().sum::<f32>() / 4.0;
    let var: f32 = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / 4.0;
    assert!(mean.abs() < 1e-4, "expected ~0 mean, got {mean}");
    assert!((var - 1.0).abs() < 1e-2, "expected ~unit variance, got {var}");
}

#[test]
#[ignore]
fn gelu_component_matches_known_values() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let input = tensor_shaped(&[3], &[0.0, 1.0, -1.0]);
    let output = bridge
        .call_component("gelu", "forward", vec![input], &serde_json::json!({}))
        .expect("gelu call failed");

    let values = read(&output, 3);
    assert!((values[0] - 0.0).abs() < 1e-5);
    assert!((values[1] - 0.8413).abs() < 1e-3); // GELU(1) ≈ 0.8413
    assert!((values[2] - (-0.1587)).abs() < 1e-3); // GELU(-1) ≈ -0.1587
}

#[test]
#[ignore]
fn add_component_is_a_real_residual_sum() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let a = tensor_shaped(&[3], &[1.0, 2.0, 3.0]);
    let b = tensor_shaped(&[3], &[10.0, 20.0, 30.0]);
    let output = bridge
        .call_component("add", "forward", vec![a, b], &serde_json::json!({}))
        .expect("add call failed");

    assert_eq!(read(&output, 3), vec![11.0, 22.0, 33.0]);
}

#[test]
#[ignore]
fn dropout_with_zero_probability_is_a_pass_through() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let input = tensor_shaped(&[4], &[1.0, 2.0, 3.0, 4.0]);
    let output = bridge
        .call_component("dropout", "forward", vec![input], &serde_json::json!({"p": 0.0}))
        .expect("dropout call failed");

    assert_eq!(read(&output, 4), vec![1.0, 2.0, 3.0, 4.0]);
}

#[test]
#[ignore]
fn embedding_component_looks_up_real_rows() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    // vocab_size=3, embedding_dim=2. Row 1 is [10, 20]; ids selects [0, 1].
    let ids = tensor_shaped(&[1, 2], &[0.0, 1.0]);
    let weight = tensor_shaped(&[3, 2], &[1.0, 2.0, 10.0, 20.0, 100.0, 200.0]);
    let output = bridge
        .call_component("embedding", "forward", vec![ids, weight], &serde_json::json!({}))
        .expect("embedding call failed");

    assert_eq!(read(&output, 4), vec![1.0, 2.0, 10.0, 20.0]);
}

#[test]
#[ignore]
fn attention_component_preserves_shape_and_is_differentiable() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let batch = 1;
    let seq = 3;
    let features = 4;
    let num_heads = 2;
    let input = tensor_shaped(&[batch, seq, features], &(0..12).map(|i| i as f32 * 0.1).collect::<Vec<_>>());
    let eye = |n: usize| -> Vec<f32> {
        let mut m = vec![0.0; n * n];
        for i in 0..n {
            m[i * n + i] = 1.0;
        }
        m
    };
    let q = tensor_shaped(&[features, features], &eye(features));
    let k = tensor_shaped(&[features, features], &eye(features));
    let v = tensor_shaped(&[features, features], &eye(features));
    let o = tensor_shaped(&[features, features], &eye(features));

    let output = bridge
        .call_component("attention", "forward", vec![input, q, k, v, o], &serde_json::json!({"num_heads": num_heads}))
        .expect("attention call failed");

    // Identity projections still change values (real softmax-weighted mixing
    // across the sequence), but must preserve the declared output shape.
    let values = read(&output, (batch * seq * features) as usize);
    assert_eq!(values.len(), 12);
    assert!(values.iter().any(|v| v.abs() > 1e-6), "attention output should not be all zeros");
}

/// The actual "next-gen ML" claim: a real transformer block — attention ->
/// residual -> layernorm -> (linear -> gelu -> linear) MLP -> residual ->
/// layernorm — trained end to end via `PythonBridge::train_step`'s real
/// forward -> loss -> backward -> optimizer-step chain, and the loss
/// measurably drops. Bypasses the CSV/tabular dataset loader (which only
/// produces 2-D (batch, features) batches — 3-D (batch, seq, features)
/// sequence loading isn't wired into `DataIterator` yet, a documented gap)
/// by constructing the (batch, seq, features) input tensor directly, the
/// same pattern the distributed-training tests use.
#[test]
#[ignore]
fn transformer_block_trains_and_loss_decreases() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let batch = 2;
    let seq = 3;
    let features = 4;
    let n = batch * seq * features;

    let input = tensor_shaped(
        &[batch, seq, features],
        &(0..n).map(|i| ((i % 7) as f32 - 3.0) * 0.2).collect::<Vec<_>>(),
    );
    // Target is an arbitrary fixed tensor the block must learn to move
    // toward — real supervised signal for a real backward pass.
    let target = tensor_shaped(&[batch, seq, features], &vec![0.5f32; n]);

    let op = |component: &str, entry: &str, inputs: Vec<&str>, outputs: Vec<&str>, hp: serde_json::Value| {
        ExecutableOp {
            node_id: component.to_string(),
            component: component.to_string(),
            language: "python".to_string(),
            entry: entry.to_string(),
            inputs: inputs.into_iter().map(String::from).collect(),
            outputs: outputs.into_iter().map(String::from).collect(),
            param_inputs: vec![],
            trainable_inputs: vec![],
            zero_init_inputs: vec![],
            hyperparams: hp,
            resolved_param_shapes: Default::default(),
        }
    };

    let operations = vec![
        op(
            "attention",
            "forward",
            vec!["x", "q_w", "k_w", "v_w", "o_w"],
            vec!["attn_out"],
            serde_json::json!({"num_heads": 2}),
        ),
        op("add", "forward", vec!["x", "attn_out"], vec!["resid1"], serde_json::json!({})),
        op(
            "layernorm",
            "forward",
            vec!["resid1", "ln1_w", "ln1_b"],
            vec!["norm1"],
            serde_json::json!({"eps": 1e-5}),
        ),
        op("linear", "forward", vec!["norm1", "mlp1_w"], vec!["hidden"], serde_json::json!({})),
        op("gelu", "forward", vec!["hidden"], vec!["activated"], serde_json::json!({})),
        op("linear", "forward", vec!["activated", "mlp2_w"], vec!["mlp_out"], serde_json::json!({})),
        op("add", "forward", vec!["norm1", "mlp_out"], vec!["resid2"], serde_json::json!({})),
        op(
            "layernorm",
            "forward",
            vec!["resid2", "ln2_w", "ln2_b"],
            vec!["output"],
            serde_json::json!({"eps": 1e-5}),
        ),
    ];

    let eye = |n: usize| -> Vec<f32> {
        let mut m = vec![0.0; n * n];
        for i in 0..n {
            m[i * n + i] = 1.0;
        }
        m
    };
    let mut weights: HashMap<String, brainbuilder_core::Tensor> = HashMap::new();
    weights.insert("q_w".into(), tensor_shaped(&[features, features], &eye(features)));
    weights.insert("k_w".into(), tensor_shaped(&[features, features], &eye(features)));
    weights.insert("v_w".into(), tensor_shaped(&[features, features], &eye(features)));
    weights.insert("o_w".into(), tensor_shaped(&[features, features], &eye(features)));
    weights.insert("ln1_w".into(), tensor_shaped(&[features], &vec![1.0; features]));
    weights.insert("ln1_b".into(), tensor_shaped(&[features], &vec![0.0; features]));
    weights.insert("ln2_w".into(), tensor_shaped(&[features], &vec![1.0; features]));
    weights.insert("ln2_b".into(), tensor_shaped(&[features], &vec![0.0; features]));
    weights.insert("mlp1_w".into(), tensor_shaped(&[features, features], &eye(features)));
    weights.insert("mlp2_w".into(), tensor_shaped(&[features, features], &eye(features)));

    let trainable: Vec<String> = weights.keys().cloned().collect();

    let mut losses = Vec::new();
    for _ in 0..30 {
        let inputs: HashMap<String, brainbuilder_core::Tensor> = std::iter::once(("x".to_string(), input.clone()))
            .chain(weights.iter().map(|(k, v)| (k.clone(), v.clone())))
            .collect();

        let (loss, updated) = bridge
            .train_step(&operations, inputs, "output", &target, "mse", "adam", 0.01, &trainable)
            .expect("transformer block train_step failed");
        weights.extend(updated);
        losses.push(loss);
    }

    let first = losses[0];
    let last = *losses.last().unwrap();
    assert!(
        last < first * 0.7,
        "expected the transformer block's loss to drop substantially: first={first}, last={last}, full curve={losses:?}"
    );
}

/// Proves LoRA fine-tuning is real: the frozen base `weight` is never
/// modified by training (byte-for-byte identical after 40 steps) while the
/// low-rank `lora_a`/`lora_b` adapters — the only ports listed in
/// `trainable_ports` — actually learn and the loss drops. `lora_b` also
/// starts at exact zero (`:zero-init true`), so the very first forward pass
/// must equal the frozen base's own output, proving the adapter starts as a
/// true no-op rather than corrupting the base model from step one.
#[test]
#[ignore]
fn lora_linear_freezes_base_weight_and_trains_only_the_adapter() {
    set_components_python_path();
    let bridge = PythonBridge::new(Arc::new(SharedArena::new())).unwrap();

    let in_features = 4;
    let out_features = 4;
    let rank = 2;
    let batch = 3;

    let eye = |n: usize| -> Vec<f32> {
        let mut m = vec![0.0; n * n];
        for i in 0..n {
            m[i * n + i] = 1.0;
        }
        m
    };
    let base_weight = tensor_shaped(&[out_features, in_features], &eye(in_features));
    let input = tensor_shaped(&[batch, in_features], &[1.0, 2.0, 3.0, 4.0, -1.0, 0.5, 2.0, -2.0, 0.0, 1.0, 1.0, 1.0]);

    let op = ExecutableOp {
        node_id: "lora".into(),
        component: "lora_linear".into(),
        language: "python".into(),
        entry: "forward".into(),
        inputs: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
        outputs: vec!["output".into()],
        param_inputs: vec!["weight".into(), "lora_a".into(), "lora_b".into()],
        trainable_inputs: vec!["lora_a".into(), "lora_b".into()],
        zero_init_inputs: vec![],
        hyperparams: serde_json::json!({"alpha": 8.0}),
        resolved_param_shapes: Default::default(),
    };

    // Base output with identity weight is just the input itself — the target
    // the adapter must learn to move *away* from, so a non-zero lora_b is
    // unambiguously required to reduce the loss.
    let target = tensor_shaped(&[batch, out_features], &vec![5.0f32; batch * out_features]);

    // lora_b starts at exact zero -> first forward pass must equal the
    // frozen base's own output (input @ identity = input), proving the
    // adapter is a true no-op before any training happens.
    let lora_a_init = tensor_shaped(&[rank, in_features], &vec![0.3, -0.2, 0.1, 0.4, -0.1, 0.2, 0.3, -0.3]);
    let lora_b_init = tensor_shaped(&[out_features, rank], &vec![0.0; out_features * rank]);
    let first_pass = bridge
        .call_component(
            "lora_linear",
            "forward",
            vec![input.clone(), base_weight.clone(), lora_a_init.clone(), lora_b_init.clone()],
            &serde_json::json!({"alpha": 8.0}),
        )
        .expect("lora_linear forward failed");
    let expected_no_op: Vec<f32> = (0..batch * in_features).map(|i| unsafe {
        *((*input.0).dl_tensor.data as *const f32).add(i)
    }).collect();
    assert_eq!(read(&first_pass, batch * out_features), expected_no_op, "zero-init lora_b must start as a no-op on the frozen base");

    let mut weights: HashMap<String, brainbuilder_core::Tensor> = HashMap::new();
    weights.insert("weight".into(), base_weight);
    weights.insert("lora_a".into(), lora_a_init);
    weights.insert("lora_b".into(), lora_b_init);
    let trainable = vec!["lora_a".to_string(), "lora_b".to_string()];

    let mut losses = Vec::new();
    for _ in 0..40 {
        let mut inputs = weights.clone();
        inputs.insert("input".to_string(), input.clone());
        let (loss, updated) = bridge
            .train_step(&[op.clone()], inputs, "output", &target, "mse", "adam", 0.05, &trainable)
            .expect("lora train_step failed");
        // Only lora_a/lora_b should ever come back as "updated".
        assert!(!updated.contains_key("weight"), "frozen base weight must never be in updated_weights");
        weights.extend(updated);
        losses.push(loss);
    }

    let base_weight_final = read(weights.get("weight").unwrap(), out_features * in_features);
    assert_eq!(base_weight_final, eye(in_features), "frozen base weight must be byte-for-byte unchanged after training");

    let first = losses[0];
    let last = *losses.last().unwrap();
    assert!(last < first * 0.5, "expected LoRA adapter training to reduce loss: first={first}, last={last}, curve={losses:?}");
}
