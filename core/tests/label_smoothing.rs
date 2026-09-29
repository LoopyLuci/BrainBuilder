// Proves the `label_smoothing` training hyperparameter has a real effect on
// the actual trained checkpoint — not just that it's threaded through and
// accepted. Trains the same seeded single-node `linear` classifier twice on
// the same, deliberately perfectly-separable data, same lr/epochs, differing
// only in `label_smoothing`: 0.0 (off) vs. 0.3. With smoothing off,
// cross_entropy on perfectly-separable data has no true minimum — pushing
// logits (and so the weights) to ever-larger magnitude always reduces loss
// further, so a fixed epoch budget drives the weight norm up substantially.
// With smoothing on, the loss's true minimum caps out at a finite
// confidence level well short of "infinite", so the same epoch budget
// can't (and doesn't need to) drive the weights nearly as far. Ignored by
// default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::orchestrator::Orchestrator;

fn graph_with_label_smoothing(graph_id: &str, csv_path: &std::path::Path, label_smoothing: f64) -> BBIRGraph {
    BBIRGraph {
        schema_version: 1,
        graph_id: graph_id.to_string(),
        name: graph_id.to_string(),
        nodes: vec![BBIRNode {
            id: "n1".to_string(),
            component: "linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({"in_features": 2, "out_features": 2}),
            ports: PortInfo {
                input_ports: vec!["input".to_string(), "weight".to_string()],
                output_ports: vec!["output".to_string()],
            },
            position: None,
        }],
        edges: Vec::<BBIREdge>::new(),
        training: Some(TrainingConfig {
            loss: "cross_entropy".to_string(),
            optimizer: "sgd".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.5, "epochs": 60, "label_smoothing": label_smoothing}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                // One batch covering the whole file: every epoch is a real,
                // identical-shape full-batch step.
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
fn label_smoothing_measurably_shrinks_the_trained_weights_on_separable_data() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Same seed for both runs — isolates label_smoothing's effect from
    // random weight-init variance, which would otherwise confound the
    // comparison.
    std::env::set_var("BRAINBUILDER_SEED", "2024");

    let csv_dir = std::env::temp_dir().join("bb_label_smoothing");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    // Perfectly linearly separable by x1: class 1 whenever x1 > 0, else
    // class 0 — cross_entropy on this data has no finite-weight minimum
    // without smoothing, which is exactly the scenario that makes the
    // comparison meaningful.
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 9.5) * 0.3;
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        let y = if x1 > 0.0 { 1.0 } else { 0.0 };
        csv.push_str(&format!("{x1},{x2},{y:.1}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let bridge = brainbuilder_core::interop::python::PythonBridge::new(std::sync::Arc::new(
        brainbuilder_core::interop::arena::SharedArena::new(),
    ))
    .unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let off_id = "label-smoothing-off-test";
    let off_graph = graph_with_label_smoothing(off_id, &csv_path, 0.0);
    let off_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{off_id}.pt"));
    std::fs::remove_file(&off_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(off_graph.clone())).expect("unsmoothed training failed");
    let off_weights = bridge.load_state_dict(&off_checkpoint).expect("failed to load unsmoothed checkpoint");
    let off_norm = weight_l2_norm(&off_weights, "n1:weight");

    let on_id = "label-smoothing-on-test";
    let on_graph = graph_with_label_smoothing(on_id, &csv_path, 0.3);
    let on_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{on_id}.pt"));
    std::fs::remove_file(&on_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(on_graph.clone())).expect("smoothed training failed");
    let on_weights = bridge.load_state_dict(&on_checkpoint).expect("failed to load smoothed checkpoint");
    let on_norm = weight_l2_norm(&on_weights, "n1:weight");

    assert!(
        on_norm < off_norm * 0.9,
        "label smoothing should measurably cap how far training drives the weights on perfectly-separable \
         data: off (smoothing=0.0)={off_norm}, on (smoothing=0.3)={on_norm} (expected on < {} of off)",
        off_norm * 0.9
    );

    std::fs::remove_file(&off_checkpoint).ok();
    std::fs::remove_file(&on_checkpoint).ok();
}
