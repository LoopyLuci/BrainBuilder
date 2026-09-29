// Proves the `momentum` training hyperparameter has a real effect on the
// actual trained checkpoint — not just that it's threaded through and
// accepted. Trains the same seeded single-node `linear` graph twice, same
// data, same (deliberately small) learning rate and epoch count, one batch
// per epoch, differing only in `momentum`: 0.0 (off) vs. 0.9. The data's
// gradient direction is consistent step to step (a simple linear target),
// so momentum's accumulated velocity should carry the weight measurably
// further toward the target over the same small-step epoch budget than
// plain SGD manages on its own. Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::orchestrator::Orchestrator;

fn graph_with_momentum(graph_id: &str, csv_path: &std::path::Path, momentum: f64) -> BBIRGraph {
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
            // Deliberately small lr: plain SGD should only creep slowly
            // toward the target over this epoch budget, leaving clear room
            // for momentum's accumulated velocity to measurably outpace it.
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 15, "momentum": momentum}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                // One batch covering the whole file: epoch count == step
                // count, so the two runs' schedules are directly comparable.
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
fn momentum_measurably_accelerates_convergence_over_plain_sgd() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Same seed for both runs — isolates momentum's effect from random
    // weight-init variance, which would otherwise confound the comparison.
    std::env::set_var("BRAINBUILDER_SEED", "31337");

    let csv_dir = std::env::temp_dir().join("bb_momentum");
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

    let off_id = "momentum-off-test";
    let off_graph = graph_with_momentum(off_id, &csv_path, 0.0);
    let off_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{off_id}.pt"));
    std::fs::remove_file(&off_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(off_graph.clone())).expect("plain SGD training failed");
    let off_weights = bridge.load_state_dict(&off_checkpoint).expect("failed to load plain-SGD checkpoint");
    let off_norm = weight_l2_norm(&off_weights, "n1:weight");

    let on_id = "momentum-on-test";
    let on_graph = graph_with_momentum(on_id, &csv_path, 0.9);
    let on_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{on_id}.pt"));
    std::fs::remove_file(&on_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(on_graph.clone())).expect("momentum training failed");
    let on_weights = bridge.load_state_dict(&on_checkpoint).expect("failed to load momentum checkpoint");
    let on_norm = weight_l2_norm(&on_weights, "n1:weight");

    assert!(
        on_norm > off_norm * 1.2,
        "momentum should measurably accelerate how far training moves the weights over the same small-step \
         epoch budget: off (momentum=0.0)={off_norm}, on (momentum=0.9)={on_norm} (expected on > {} of off)",
        off_norm * 1.2
    );

    std::fs::remove_file(&off_checkpoint).ok();
    std::fs::remove_file(&on_checkpoint).ok();
}
