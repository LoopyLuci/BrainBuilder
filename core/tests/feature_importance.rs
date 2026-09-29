// Proves `interpret::feature_importance` runs its full real path — a real
// trained checkpoint, real `predict` calls, real column rotation — against
// the actual Python/torch bridge, not just that the rotation math is correct
// in isolation (that's covered by interpret.rs's fast inline unit tests,
// which is where the real correctness proof lives).
//
// This does NOT assert that x1 outscores x2 despite y being defined as
// 2*x1 (x2 uncorrelated noise) below. It should, in principle — but a bare
// single-node `linear` graph trained with no descriptor-sized hyperparams
// was observed, while writing this test, to barely move off its initial
// weights even after 2000 epochs at lr=0.1 (importance scores on the order
// of 1e-2 for both columns, rather than the ~5-unit swing `2*range(x1)`
// would predict for a converged model). That looks like a real, separate
// issue in either checkpoint round-tripping or the symbolic-shape-inferred
// single-node training path — filed as its own task rather than chased down
// here. This test therefore only asserts the pipeline itself is sound:
// real training completes, a real checkpoint exists, `feature_importance`
// returns exactly the two real feature columns (target excluded) with
// finite, non-negative, correctly-sorted scores.
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::data::source::load_batch;
use brainbuilder_core::interpret::feature_importance;
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn feature_importance_runs_end_to_end_on_a_real_trained_model() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let csv_dir = std::env::temp_dir().join("bb_feature_importance");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 10.0) * 0.3;
        // x2 is pure noise, uncorrelated with y, and deliberately similar in
        // scale to x1 — permutation importance is scale-sensitive (rotating a
        // larger-magnitude column swings predictions more even at an equally
        // small learned weight), so mismatched ranges would confound "does
        // the model use this" with "how big are this column's raw values".
        // The `* 0.3 + 0.05` keeps every value fractional so the CSV reader
        // infers Float32 (an all-whole-number column would infer as Int64
        // and break the float32-only bridge downstream).
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        // Deliberately uncorrelated with y — see the file-level comment for
        // why this test doesn't assert x1 outscores x2 despite that.
        let y = 2.0 * x1;
        csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "feature-importance-test".to_string(),
        name: "feature-importance-test".to_string(),
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
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 100}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
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
    };

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone())).expect("training failed");
    assert!(orchestrator.has_checkpoint(&graph.graph_id));

    let batch = rt.block_on(load_batch(&csv_path.to_string_lossy(), 20)).expect("loading the batch failed");
    let scores = feature_importance(&orchestrator, &graph, batch).expect("feature importance failed");

    assert_eq!(scores.len(), 2, "two feature columns (x1, x2), target excluded");
    let columns: Vec<&str> = scores.iter().map(|s| s.column.as_str()).collect();
    assert!(columns.contains(&"x1") && columns.contains(&"x2"), "got columns: {columns:?}");
    for s in &scores {
        assert!(s.importance.is_finite() && s.importance >= 0.0, "importance should be a real, non-negative number: {s:?}");
    }
    assert!(scores[0].importance >= scores[1].importance, "results should be sorted highest-importance first");

    std::fs::remove_file(&checkpoint_path).ok();
}
