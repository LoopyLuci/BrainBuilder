// Proves the `weight_decay` training hyperparameter has a real effect on
// the actual trained checkpoint — not just that it's threaded through and
// accepted. Trains the same seeded single-node `linear` graph twice, on the
// same data, same learning rate and epoch count, differing only in
// `weight_decay`: 0.0 (off) vs. a deliberately large 0.5. Weight decay pulls
// every weight a little toward zero on every step regardless of the
// gradient signal, so — with the same random init (seeded) and the same
// number of real steps — the decayed run's final weight L2 norm must come
// out smaller than the undecayed run's. Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::orchestrator::Orchestrator;

fn graph_with_weight_decay(graph_id: &str, csv_path: &std::path::Path, weight_decay: f64) -> BBIRGraph {
    BBIRGraph {
        schema_version: 1,
        graph_id: graph_id.to_string(),
        name: graph_id.to_string(),
        nodes: vec![BBIRNode {
            id: "n1".to_string(),
            component: "linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({}),
            ports: PortInfo {
                input_ports: vec!["input".to_string(), "weight".to_string()],
                output_ports: vec!["output".to_string()],
            },
            position: None,
        }],
        edges: Vec::<BBIREdge>::new(),
        training: Some(TrainingConfig {
            loss: "mse".to_string(),
            optimizer: "sgd".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.05, "epochs": 50, "weight_decay": weight_decay}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                // One batch per epoch: 50 real optimizer steps, each one
                // applying weight_decay's shrink.
                batch_size: 20,
                preprocessing: Vec::new(),
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

fn weight_l2_norm(weights: &std::collections::HashMap<String, brainbuilder_core::Tensor>, key: &str) -> f32 {
    let t = weights.get(key).unwrap_or_else(|| panic!("checkpoint missing expected key `{key}`"));
    let values = unsafe {
        let dl = &(*t.0).dl_tensor;
        let mut n = 1usize;
        for i in 0..dl.ndim {
            n *= *dl.shape.offset(i as isize) as usize;
        }
        std::slice::from_raw_parts(dl.data as *const f32, n).to_vec()
    };
    values.iter().map(|v| v * v).sum::<f32>().sqrt()
}

#[test]
#[ignore]
fn weight_decay_measurably_shrinks_the_trained_weights() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Same seed for both runs — isolates weight_decay's effect from random
    // weight-init variance, which would otherwise confound the comparison.
    std::env::set_var("BRAINBUILDER_SEED", "1234");

    let csv_dir = std::env::temp_dir().join("bb_weight_decay");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 10.0) * 0.3;
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        let y = 2.0 * x1;
        csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let bridge = brainbuilder_core::interop::python::PythonBridge::new(std::sync::Arc::new(
        brainbuilder_core::interop::arena::SharedArena::new(),
    ))
    .unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let off_id = "weight-decay-off-test";
    let off_graph = graph_with_weight_decay(off_id, &csv_path, 0.0);
    let off_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{off_id}.pt"));
    std::fs::remove_file(&off_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(off_graph.clone())).expect("undecayed training failed");
    let off_weights = bridge.load_state_dict(&off_checkpoint).expect("failed to load undecayed checkpoint");
    let off_norm = weight_l2_norm(&off_weights, "n1:weight");

    let on_id = "weight-decay-on-test";
    let on_graph = graph_with_weight_decay(on_id, &csv_path, 0.5);
    let on_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{on_id}.pt"));
    std::fs::remove_file(&on_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(on_graph.clone())).expect("decayed training failed");
    let on_weights = bridge.load_state_dict(&on_checkpoint).expect("failed to load decayed checkpoint");
    let on_norm = weight_l2_norm(&on_weights, "n1:weight");

    assert!(
        on_norm < off_norm * 0.9,
        "weight decay should measurably shrink the trained weight's L2 norm: \
         off={off_norm}, on={on_norm} (expected on < {} of off)",
        off_norm * 0.9
    );

    std::fs::remove_file(&off_checkpoint).ok();
    std::fs::remove_file(&on_checkpoint).ok();
}
