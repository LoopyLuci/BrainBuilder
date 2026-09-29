// Proves the `grad_clip` training hyperparameter has a real effect on the
// actual trained checkpoint — not just that it's threaded through and
// accepted. Trains the same seeded single-node `linear` graph twice, one
// real optimizer step each, on the same data and same (deliberately large)
// learning rate, differing only in `grad_clip`: 0.0 (off) vs. a small 0.5.
// The data is crafted so the very first step's gradient is huge (targets
// are ~500x the input, so an untrained near-zero weight is wildly wrong).
// Without clipping, that huge gradient times a large learning rate drives
// the weight far from its start. With clipping, the gradient's L2 norm is
// capped before the step is applied, so the same step must land much
// closer to the seeded initial weight. Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::orchestrator::Orchestrator;

fn graph_with_grad_clip(graph_id: &str, csv_path: &std::path::Path, grad_clip: f64) -> BBIRGraph {
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
            // A large learning rate and exactly one real step (one batch,
            // one epoch) so the whole run's weight movement comes from a
            // single, deliberately oversized gradient — nothing gets a
            // chance to "settle" that would blur the clipped-vs-unclipped
            // comparison.
            hyperparams: serde_json::json!({"lr": 2.0, "epochs": 1, "grad_clip": grad_clip}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                // One batch covering the whole file: exactly one optimizer
                // step for the whole run.
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
fn grad_clip_measurably_bounds_the_first_step() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Same seed for both runs — isolates grad_clip's effect from random
    // weight-init variance, which would otherwise confound the comparison.
    std::env::set_var("BRAINBUILDER_SEED", "4242");

    let csv_dir = std::env::temp_dir().join("bb_grad_clip");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    // Targets are ~500x the inputs: with a near-zero random init, the very
    // first prediction is wildly wrong, so the first step's raw gradient is
    // large — exactly the scenario grad_clip exists to temper.
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 10.0) * 0.3;
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        let y = 500.0 * x1;
        csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let bridge = brainbuilder_core::interop::python::PythonBridge::new(std::sync::Arc::new(
        brainbuilder_core::interop::arena::SharedArena::new(),
    ))
    .unwrap();
    let rt = tokio::runtime::Runtime::new().unwrap();

    let off_id = "grad-clip-off-test";
    let off_graph = graph_with_grad_clip(off_id, &csv_path, 0.0);
    let off_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{off_id}.pt"));
    std::fs::remove_file(&off_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(off_graph.clone())).expect("unclipped training failed");
    let off_weights = bridge.load_state_dict(&off_checkpoint).expect("failed to load unclipped checkpoint");
    let off_norm = weight_l2_norm(&off_weights, "n1:weight");

    let on_id = "grad-clip-on-test";
    let on_graph = graph_with_grad_clip(on_id, &csv_path, 0.5);
    let on_checkpoint = components_dir.parent().unwrap().join("checkpoints").join(format!("{on_id}.pt"));
    std::fs::remove_file(&on_checkpoint).ok();
    rt.block_on(orchestrator.execute_graph(on_graph.clone())).expect("clipped training failed");
    let on_weights = bridge.load_state_dict(&on_checkpoint).expect("failed to load clipped checkpoint");
    let on_norm = weight_l2_norm(&on_weights, "n1:weight");

    assert!(
        off_norm.is_finite() && on_norm.is_finite(),
        "both runs should produce finite weights: off={off_norm}, on={on_norm}"
    );
    assert!(
        on_norm < off_norm * 0.5,
        "clipping the gradient should measurably bound the first step's weight movement: \
         off={off_norm}, on={on_norm} (expected on < {} of off)",
        off_norm * 0.5
    );

    std::fs::remove_file(&off_checkpoint).ok();
    std::fs::remove_file(&on_checkpoint).ok();
}
