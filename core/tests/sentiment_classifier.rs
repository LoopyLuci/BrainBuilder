// Proves the bundled sentiment-classifier example (gui/examples/sentiment.*)
// isn't just a graph that runs without crashing — it actually learns to
// separate positive from negative reviews. Trains the real bundled example
// end to end via `Orchestrator`, then re-derives the exact same real
// bag-of-words batches training used (`data::source::load_dataset` is a pure
// function of the dataset file + config, so calling it again after training
// reproduces the identical feature vectors and integer-class targets) and
// runs each one back through the trained checkpoint via `predict`, comparing
// argmax(logits) to the true class. Chance on 2 balanced classes is 50%;
// asserting well above that on data the model was trained on is a real,
// low-flakiness proof of learning (as opposed to merely "didn't error").
// Ignored by default — needs a real Python/torch worker.
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn bundled_sentiment_example_trains_and_classifies_real_reviews() {
    let gui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui");
    let components_dir = gui_dir.join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let example_edn = std::fs::read_to_string(gui_dir.join("examples/sentiment.bbir.edn"))
        .expect("bundled sentiment example graph missing — run `cargo test --test gen_sentiment_example -- --ignored`");
    let mut graph = BBIRGraph::from_edn(&example_edn).expect("bundled sentiment example graph failed to parse");

    // `path_or_uri` is cwd-relative in the bundled file (matches the real
    // app's convention); patch to an absolute path so this test doesn't
    // depend on the process's working directory.
    let training = graph.training.as_mut().expect("bundled example should carry a training config");
    training.data_source.path_or_uri = gui_dir
        .join("examples/sentiment.csv")
        .to_string_lossy()
        .to_string();

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");

    let checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();
    assert!(!orchestrator.has_checkpoint(&graph.graph_id), "checkpoint should not exist before training");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone())).expect("training the sentiment example failed");
    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should have saved a checkpoint");

    // Re-derive the exact same real bag-of-words batches training consumed:
    // `load_dataset` is a pure function of the (unchanged) CSV + config, so
    // this reproduces identical feature/target columns without needing a
    // separate held-out file.
    let mut data = rt
        .block_on(brainbuilder_core::data::source::load_dataset(graph.training.as_ref().unwrap()))
        .expect("failed to reload the real bag-of-words dataset for evaluation");

    let mut correct = 0usize;
    let mut total = 0usize;
    while let Some(batch) = data.next().expect("dataset iteration failed") {
        let num_rows = batch.num_rows();
        let num_cols = batch.num_columns();
        let targets = column_f32(&batch, num_cols - 1);
        let features_only = drop_last_column(&batch);

        let tensors = orchestrator
            .predict(graph.clone(), features_only)
            .expect("predict failed even though a checkpoint exists");

        // `forward` returns every named intermediate tensor (echoed inputs,
        // weights, and real op outputs) in unspecified order — the real
        // per-row logits are identified by shape, same convention
        // `interpret.rs::predict_flat` documents: the smallest tensor whose
        // element count divides evenly by the row count.
        let mut best: Option<Vec<f32>> = None;
        for t in &tensors {
            let (_, values) = brainbuilder_core::interop::dlpack_support::tensor_to_vec_f32(t)
                .expect("output tensor should be float32");
            if num_rows == 0 || values.is_empty() || values.len() % num_rows != 0 {
                continue;
            }
            if best.as_ref().map_or(true, |b| values.len() < b.len()) {
                best = Some(values);
            }
        }
        let logits = best.expect("predict should return an output tensor shaped for this batch's rows");
        let num_classes = logits.len() / num_rows;
        assert_eq!(num_classes, 2, "sentiment example has 2 classes (negative, positive)");

        for row in 0..num_rows {
            let row_logits = &logits[row * num_classes..(row + 1) * num_classes];
            let predicted = row_logits
                .iter()
                .enumerate()
                .max_by(|a, b| a.1.partial_cmp(b.1).unwrap())
                .map(|(i, _)| i as f32)
                .unwrap();
            if (predicted - targets[row]).abs() < 0.5 {
                correct += 1;
            }
            total += 1;
        }
    }

    assert_eq!(total, 60, "the bundled sentiment.csv has 60 rows");
    let accuracy = correct as f32 / total as f32;
    assert!(
        accuracy >= 0.9,
        "sentiment classifier should have genuinely learned to separate positive/negative reviews \
         (chance = 0.5), got accuracy={accuracy} ({correct}/{total})"
    );
    println!("sentiment classifier training-set accuracy: {accuracy} ({correct}/{total})");

    std::fs::remove_file(&checkpoint_path).ok();
}

fn column_f32(batch: &arrow::record_batch::RecordBatch, idx: usize) -> Vec<f32> {
    use arrow::array::Float32Array;
    batch
        .column(idx)
        .as_any()
        .downcast_ref::<Float32Array>()
        .expect("bag-of-words columns are float32")
        .values()
        .to_vec()
}

/// Drops the last column (the target, by this codebase's established
/// convention) — `predict` expects exactly the feature columns the model was
/// trained on, not the target it was trained to predict (see
/// `interpret.rs::drop_last_column`, duplicated here since it's private to
/// that module and this is a separate integration-test crate).
fn drop_last_column(batch: &arrow::record_batch::RecordBatch) -> arrow::record_batch::RecordBatch {
    let num_cols = batch.num_columns();
    let schema = batch.schema();
    let fields: Vec<arrow::datatypes::Field> =
        schema.fields().iter().take(num_cols - 1).map(|f| f.as_ref().clone()).collect();
    let columns: Vec<arrow::array::ArrayRef> = batch.columns()[..num_cols - 1].to_vec();
    arrow::record_batch::RecordBatch::try_new(std::sync::Arc::new(arrow::datatypes::Schema::new(fields)), columns)
        .expect("dropping the target column should always succeed")
}
