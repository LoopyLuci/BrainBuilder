// Proves `batch_predict::run_batch_predict` runs its full real path — a real
// trained checkpoint, streamed chunks from a real CSV file, real `predict`
// calls per chunk — and writes a real CSV file back out, not just that the
// escaping helper is correct in isolation (covered by batch_predict.rs's
// fast inline unit tests).
//
// Unlike `feature_importance.rs`'s test file, the input CSV here holds only
// the two feature columns the trained `linear` node expects (no target
// column) — the same shape any real deployment input would have, and it
// sidesteps the known `predict`-doesn't-drop-the-target-column shape
// mismatch documented in `interpret.rs` (not this test's concern to chase).
use brainbuilder_core::batch_predict::run_batch_predict;
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn run_batch_predict_writes_one_prediction_per_row_for_a_real_trained_model() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let work_dir = std::env::temp_dir().join("bb_batch_predict");
    std::fs::create_dir_all(&work_dir).unwrap();

    // Training file: x1, x2, y (y = 2*x1, x2 is noise) — same shape used
    // elsewhere in this suite for a single-`linear`-node graph.
    let train_path = work_dir.join("train.csv");
    let mut train_csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 10.0) * 0.3;
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        let y = 2.0 * x1;
        train_csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&train_path, train_csv).unwrap();

    // Inference file: feature columns only (x1, x2), no target — a real
    // batch-inference input, distinct rows from the training file.
    let infer_path = work_dir.join("infer.csv");
    let mut infer_csv = String::from("x1,x2\n");
    for i in 0..7 {
        let x1 = (i as f32 - 3.0) * 0.5;
        let x2 = (i as f32) * 0.1;
        infer_csv.push_str(&format!("{x1},{x2}\n"));
    }
    std::fs::write(&infer_path, infer_csv).unwrap();

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "batch-predict-test".to_string(),
        name: "batch-predict-test".to_string(),
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
                path_or_uri: train_path.to_string_lossy().to_string(),
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

    let output_path = work_dir.join("predictions.csv");
    std::fs::remove_file(&output_path).ok();
    let rows_written = rt
        .block_on(run_batch_predict(&orchestrator, &graph, &infer_path.to_string_lossy(), &output_path))
        .expect("batch predict failed");

    assert_eq!(rows_written, 7, "one prediction per input row");

    let written = std::fs::read_to_string(&output_path).expect("reading the output CSV failed");
    let mut lines = written.lines();
    assert_eq!(lines.next(), Some("x1,x2,prediction"), "header should be feature columns + prediction");
    let data_lines: Vec<&str> = lines.collect();
    assert_eq!(data_lines.len(), 7, "one CSV data row per input row");
    for line in &data_lines {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 3, "x1,x2,prediction");
        let prediction: f32 = fields[2].parse().expect("prediction field should parse as a real number");
        assert!(prediction.is_finite(), "prediction should be a real, finite number");
    }

    std::fs::remove_file(&checkpoint_path).ok();
    std::fs::remove_file(&output_path).ok();
}

/// Regression test for a real bug: `run_batch_predict` used to index
/// `predict_flat`'s output as if it held exactly one scalar per row
/// (`predictions.get(row)`), which is only true for a regression model. A
/// classifier's real output is `[batch, num_classes]` — for a 2-class model
/// this used to silently write `predictions[0]` for row 0, `predictions[1]`
/// (row 0's *other* class logit) for row 1, and so on, class-interleaved and
/// completely misaligned with the actual rows. This trains a real, perfectly
/// separable 2-class `linear` classifier and checks the CSV's `prediction`
/// column holds the correct class index (0 or 1) per row, not a raw logit.
#[test]
#[ignore]
fn run_batch_predict_writes_the_argmax_class_for_a_real_trained_classifier() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let work_dir = std::env::temp_dir().join("bb_batch_predict_classifier");
    std::fs::create_dir_all(&work_dir).unwrap();

    // Perfectly linearly separable by x1: class 1 whenever x1 > 0, else
    // class 0 (same shape `label_smoothing.rs` uses) — deterministic enough
    // that a real trained model's predicted class is unambiguous per row.
    let train_path = work_dir.join("train.csv");
    let mut train_csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 9.5) * 0.3;
        let x2 = (((i * 37) % 11) as f32 - 5.0) * 0.3 + 0.05;
        let y = if x1 > 0.0 { 1.0 } else { 0.0 };
        train_csv.push_str(&format!("{x1},{x2},{y:.1}\n"));
    }
    std::fs::write(&train_path, train_csv).unwrap();

    // Inference file: well clear of the decision boundary on both sides, so
    // the expected class per row is unambiguous. Explicit `.1` precision
    // matters here, not just style: a whole-number value like `-3.0` prints
    // as `-3` via Rust's default float Display, and if every row in a CSV
    // column happens to look integral, DataFusion's schema inference infers
    // that column as Int64 instead of Float64/Float32 — `stack_columns_into_
    // matrix` (arrow_bridge.rs) requires every column be float, so a stray
    // all-whole-number column would fail with a real (if here, self-
    // inflicted) "requires a float32 tensor" error.
    let infer_path = work_dir.join("infer.csv");
    let mut infer_csv = String::from("x1,x2\n");
    let infer_rows: Vec<(f32, f32)> = vec![(-3.0, 0.1), (-2.0, -0.2), (2.0, 0.3), (3.0, -0.1)];
    for (x1, x2) in &infer_rows {
        infer_csv.push_str(&format!("{x1:.1},{x2:.1}\n"));
    }
    std::fs::write(&infer_path, infer_csv).unwrap();

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "batch-predict-classifier-test".to_string(),
        name: "batch-predict-classifier-test".to_string(),
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
            hyperparams: serde_json::json!({"lr": 0.5, "epochs": 60}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: train_path.to_string_lossy().to_string(),
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

    let output_path = work_dir.join("predictions.csv");
    std::fs::remove_file(&output_path).ok();
    let rows_written = rt
        .block_on(run_batch_predict(&orchestrator, &graph, &infer_path.to_string_lossy(), &output_path))
        .expect("batch predict failed");
    assert_eq!(rows_written, 4);

    let written = std::fs::read_to_string(&output_path).expect("reading the output CSV failed");
    let mut lines = written.lines();
    assert_eq!(lines.next(), Some("x1,x2,prediction"));
    let data_lines: Vec<&str> = lines.collect();
    assert_eq!(data_lines.len(), 4);

    for (line, (x1, _)) in data_lines.iter().zip(infer_rows.iter()) {
        let fields: Vec<&str> = line.split(',').collect();
        assert_eq!(fields.len(), 3);
        let predicted_class: f32 = fields[2].parse().expect("prediction field should parse as a real number");
        // A real class index (0 or 1), not a raw logit or a class-interleaved
        // value from a mis-indexed multi-class output.
        assert!(
            predicted_class == 0.0 || predicted_class == 1.0,
            "prediction should be a real class index (0 or 1), got {predicted_class}"
        );
        let expected_class = if *x1 > 0.0 { 1.0 } else { 0.0 };
        assert_eq!(
            predicted_class, expected_class,
            "row with x1={x1} should predict class {expected_class} on this perfectly-separable data, got {predicted_class}"
        );
    }

    std::fs::remove_file(&checkpoint_path).ok();
    std::fs::remove_file(&output_path).ok();
}
