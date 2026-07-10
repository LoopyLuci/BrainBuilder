// Proves the transfer-learning seam at the plan level (no torch needed): a
// real `.safetensors` file is written to disk, a graph references one of its
// tensors as a node's pretrained `weight`, and `compile()` loads that real
// tensor into the plan's `preset_weights` with the exact values from the file —
// keyed by the namespaced port the trainer binds. This is the mechanism that
// makes a frozen pretrained backbone real (the `lora_linear` base `weight` is
// non-trainable, so a seeded value stays fixed while the adapters learn).
use brainbuilder_core::bbir::{BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::interop::dlpack_support::tensor_to_vec_f32;
use brainbuilder_core::orchestrator::Orchestrator;
use brainbuilder_core::runtime::scheduler::compile;
use safetensors::tensor::TensorView;
use safetensors::Dtype;
use std::collections::HashMap;
use std::path::Path;

/// Write a real single-tensor `.safetensors` file (`backbone.weight`, shape
/// `[out, in]`) with known, non-trivial values (a spread of positive and
/// negative magnitudes, so a frozen backbone actually produces informative
/// features rather than near-zero output), through the official crate's own
/// serializer — the same binary format the loader reads.
fn write_safetensors(path: &Path, out: usize, in_: usize) -> Vec<f32> {
    let values: Vec<f32> = (0..out * in_).map(|i| ((i as f32) * 0.37).sin() * 0.8).collect();
    let bytes: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
    let view = TensorView::new(Dtype::F32, vec![out, in_], &bytes).unwrap();
    let mut map: HashMap<String, TensorView> = HashMap::new();
    map.insert("backbone.weight".to_string(), view);
    safetensors::serialize_to_file(&map, &None, path).unwrap();
    values
}

fn lora_graph(pretrained: serde_json::Value, in_features: i64, out_features: i64) -> BBIRGraph {
    BBIRGraph {
        schema_version: 1,
        graph_id: format!("transfer-{}", uuid::Uuid::new_v4()),
        name: "transfer-test".to_string(),
        nodes: vec![BBIRNode {
            id: "adapt".to_string(),
            component: "lora_linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({
                "in_features": in_features,
                "out_features": out_features,
                "rank": 2,
                "alpha": 4.0,
                "pretrained": pretrained,
            }),
            ports: PortInfo {
                input_ports: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
                output_ports: vec!["output".into()],
            },
            position: None,
        }],
        edges: vec![],
        training: Some(TrainingConfig {
            loss: "mse".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 5}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: "unused-by-compile.csv".to_string(),
                batch_size: 4,
                preprocessing: vec![],
                sequence_length: None,
                vocab_size: None,
                image_size: None,
                grayscale: None,
                text_column: None,
                label_column: None,
            },
            reproducibility: None,
        }),
    }
}

#[test]
fn compile_seeds_a_nodes_weight_from_a_real_safetensors_file() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    let dir = std::env::temp_dir().join(format!("bb_transfer_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let st_path = dir.join("backbone.safetensors");
    let expected = write_safetensors(&st_path, 4, 8); // [out=4, in=8]

    let graph = lora_graph(
        serde_json::json!({ "file": st_path.to_string_lossy(), "tensor": "backbone.weight", "port": "weight" }),
        8,
        4,
    );

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init");
    let plan = compile(&graph, &orchestrator.context).expect("compile should load the pretrained weight");

    // The pretrained base weight is present, keyed by the namespaced port.
    let seeded = plan.preset_weights.get("adapt:weight").expect("adapt:weight should be seeded");
    let (_shape, got) = tensor_to_vec_f32(seeded).expect("read seeded tensor values");
    assert_eq!(got.len(), expected.len(), "seeded weight has the file's element count");
    for (a, b) in got.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-6, "seeded value {a} should match file value {b}");
    }

    // The LoRA adapters are NOT preset — only the base weight is.
    assert!(!plan.preset_weights.contains_key("adapt:lora_a"));
    assert!(!plan.preset_weights.contains_key("adapt:lora_b"));

    std::fs::remove_dir_all(&dir).ok();
}

/// End-to-end (needs torch): the classic transfer-learning pattern — a frozen
/// pretrained backbone (a `lora_linear` whose base `weight` is seeded from a
/// real `.safetensors` file and never updated) feeding a fresh, fully
/// trainable head (`linear`). The head learns to map the backbone's fixed
/// features to the target, so the loss drops substantially. This proves the
/// seeded pretrained weights integrate into real multi-node autograd training.
#[test]
#[ignore]
fn a_frozen_pretrained_backbone_with_a_fresh_head_trains() {
    use brainbuilder_core::bbir::BBIREdge;
    use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};

    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Deterministic weight init (adapters + head) for a non-flaky CI run (see
    // _bb_worker.py::_apply_seed).
    std::env::set_var("BRAINBUILDER_SEED", "7");

    let dir = std::env::temp_dir().join(format!("bb_transfer_train_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let st_path = dir.join("backbone.safetensors");
    write_safetensors(&st_path, 4, 4); // frozen backbone projection [4,4]

    // Real CSV: 4 features + a linear target.
    let csv_path = dir.join("data.csv");
    let mut csv = String::from("x1,x2,x3,x4,y\n");
    for i in 0..40 {
        let x1 = (i as f32 - 20.0) * 0.1;
        let x2 = (i as f32 - 10.0) * 0.06;
        let x3 = (i as f32 % 7.0) * 0.2;
        let x4 = (i as f32 % 3.0) * 0.3;
        let y = 0.8 * x1 - 0.5 * x2 + 0.3 * x3;
        csv.push_str(&format!("{x1},{x2},{x3},{x4},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    // backbone (frozen pretrained lora_linear, 4->4) -> relu -> head (linear 4->1, trainable).
    let backbone = BBIRNode {
        id: "backbone".into(),
        component: "lora_linear".into(),
        label: None,
        hyperparams: serde_json::json!({
            "in_features": 4, "out_features": 4, "rank": 2, "alpha": 8.0,
            "pretrained": { "file": st_path.to_string_lossy(), "tensor": "backbone.weight", "port": "weight" }
        }),
        ports: PortInfo {
            input_ports: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
            output_ports: vec!["output".into()],
        },
        position: None,
    };
    let act = BBIRNode {
        id: "act".into(),
        component: "relu".into(),
        label: None,
        hyperparams: serde_json::json!({}),
        ports: PortInfo { input_ports: vec!["x".into()], output_ports: vec!["y".into()] },
        position: None,
    };
    let head = BBIRNode {
        id: "head".into(),
        component: "linear".into(),
        label: None,
        hyperparams: serde_json::json!({"in_features": 4, "out_features": 1}),
        ports: PortInfo { input_ports: vec!["input".into(), "weight".into()], output_ports: vec!["output".into()] },
        position: None,
    };

    let mut graph = lora_graph(serde_json::Value::Null, 4, 4);
    graph.nodes = vec![backbone, act, head];
    graph.edges = vec![
        BBIREdge { from_node: "backbone".into(), from_port: "output".into(), to_node: "act".into(), to_port: "x".into() },
        BBIREdge { from_node: "act".into(), from_port: "y".into(), to_node: "head".into(), to_port: "input".into() },
    ];
    let training = graph.training.as_mut().unwrap();
    training.hyperparams = serde_json::json!({"lr": 0.02, "epochs": 120});
    training.data_source.path_or_uri = csv_path.to_string_lossy().to_string();
    training.data_source.batch_size = 40;

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init");
    let checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint).ok();

    // Drain metrics on a background thread *while* training runs, the same way
    // the real GUI drains them into Tauri events. Draining only afterward would
    // lose the whole curve to the broadcast channel's bounded buffer (200) once
    // a run produces more steps than that — which a 300-epoch run does.
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::{Arc, Mutex};
    let collected: Arc<Mutex<Vec<MetricPoint>>> = Arc::new(Mutex::new(Vec::new()));
    let done = Arc::new(AtomicBool::new(false));
    let mut metrics_rx = subscribe_metrics();
    let collector = {
        let collected = collected.clone();
        let done = done.clone();
        std::thread::spawn(move || loop {
            match metrics_rx.try_recv() {
                Ok(p) => collected.lock().unwrap().push(p),
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => {
                    if done.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(std::time::Duration::from_millis(1));
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => {}
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => break,
            }
        })
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone())).expect("transfer-learning training failed");
    done.store(true, Ordering::SeqCst);
    collector.join().unwrap();

    let points = collected.lock().unwrap().clone();
    assert!(points.len() >= 4, "expected multiple steps, got {}", points.len());
    let first = points.first().unwrap().loss;
    let last = points.last().unwrap().loss;
    assert!(
        last < first * 0.7,
        "a fresh head on a frozen pretrained backbone should reduce loss: first={first}, last={last}"
    );

    std::fs::remove_dir_all(&dir).ok();
}

#[test]
fn compile_rejects_a_pretrained_tensor_whose_shape_doesnt_match() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    let dir = std::env::temp_dir().join(format!("bb_transfer_bad_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let st_path = dir.join("backbone.safetensors");
    write_safetensors(&st_path, 3, 8); // [out=3, in=8]

    // Node declares out_features=4 -> expected weight [4,8], but the file is [3,8].
    let graph = lora_graph(
        serde_json::json!({ "file": st_path.to_string_lossy(), "tensor": "backbone.weight" }),
        8,
        4,
    );

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init");
    let err = match compile(&graph, &orchestrator.context) {
        Ok(_) => panic!("compile should have rejected the shape-mismatched pretrained tensor"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("shape"), "expected a shape-mismatch error, got: {err}");

    std::fs::remove_dir_all(&dir).ok();
}

/// Proves the flat `pretrained_file`/`pretrained_tensor` hyperparameters
/// (declared on `lora_linear.edn` so the Inspector's generic `SchemaForm` can
/// set them on a hand-dragged node — unlike the nested `pretrained` object,
/// which only the Intent transfer-learning flow can author) load and seed the
/// real tensor exactly like the nested form does.
#[test]
fn compile_seeds_a_nodes_weight_from_flat_pretrained_hyperparams() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    let dir = std::env::temp_dir().join(format!("bb_transfer_flat_{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let st_path = dir.join("backbone.safetensors");
    let expected = write_safetensors(&st_path, 4, 8); // [out=4, in=8]

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: format!("transfer-flat-{}", uuid::Uuid::new_v4()),
        name: "transfer-flat-test".to_string(),
        nodes: vec![BBIRNode {
            id: "adapt".to_string(),
            component: "lora_linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({
                "in_features": 8,
                "out_features": 4,
                "rank": 2,
                "alpha": 4.0,
                "pretrained_file": st_path.to_string_lossy(),
                "pretrained_tensor": "backbone.weight",
            }),
            ports: PortInfo {
                input_ports: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
                output_ports: vec!["output".into()],
            },
            position: None,
        }],
        edges: vec![],
        training: None,
    };

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init");
    let plan = compile(&graph, &orchestrator.context).expect("compile should load the flat pretrained fields");

    let seeded = plan.preset_weights.get("adapt:weight").expect("adapt:weight should be seeded");
    let (_shape, got) = tensor_to_vec_f32(seeded).expect("read seeded tensor values");
    assert_eq!(got.len(), expected.len(), "seeded weight has the file's element count");
    for (a, b) in got.iter().zip(expected.iter()) {
        assert!((a - b).abs() < 1e-6, "seeded value {a} should match file value {b}");
    }

    std::fs::remove_dir_all(&dir).ok();
}

/// An empty-string `pretrained_file`/`pretrained_tensor` (the Inspector's
/// default for a `lora_linear` node nobody has pointed at a real file yet)
/// must be silently ignored, not treated as "load a file named ``" — the
/// base weight instead falls back to ordinary random initialization, exactly
/// like before these fields existed.
#[test]
fn empty_flat_pretrained_hyperparams_are_ignored() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "transfer-flat-empty-test".to_string(),
        name: "transfer-flat-empty-test".to_string(),
        nodes: vec![BBIRNode {
            id: "adapt".to_string(),
            component: "lora_linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({
                "in_features": 8,
                "out_features": 4,
                "rank": 2,
                "alpha": 4.0,
                "pretrained_file": "",
                "pretrained_tensor": "",
            }),
            ports: PortInfo {
                input_ports: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
                output_ports: vec!["output".into()],
            },
            position: None,
        }],
        edges: vec![],
        training: None,
    };

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init");
    let plan = compile(&graph, &orchestrator.context).expect("compile should succeed with empty pretrained fields");

    assert!(
        !plan.preset_weights.contains_key("adapt:weight"),
        "empty pretrained_file/pretrained_tensor should not attempt to load anything"
    );
}
