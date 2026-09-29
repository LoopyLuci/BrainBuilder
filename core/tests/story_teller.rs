// Proves the bundled story-teller example (gui/examples/story.*) — the first
// bundled example to use real self-attention — actually learns to predict
// the next word of its little 4-sentence story cycle, through the exact
// production path the GUI uses (Orchestrator::execute_graph /
// Orchestrator::predict), not a synthetic hand-built-ops test. Trains end to
// end, then re-derives the identical real (ids, target) windows training
// used (`data::source::load_dataset` is a pure function of the corpus file +
// config) and runs them back through the trained checkpoint, comparing
// argmax(logits) to the true next-token id. Chance on a real 28-word
// vocabulary is ~3.6%; a bigram (bare embedding, no attention — see
// sequence_training.rs) can't solve this corpus at all, since "the" alone
// precedes four different words across the four sentences — only a model
// that actually attends over the whole context window can. Ignored by
// default — needs a real Python/torch worker.
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn bundled_story_example_trains_and_predicts_the_next_word() {
    let gui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui");
    let components_dir = gui_dir.join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let example_edn = std::fs::read_to_string(gui_dir.join("examples/story.bbir.edn"))
        .expect("bundled story example graph missing — run `cargo test --test gen_story_example -- --ignored`");
    let mut graph = BBIRGraph::from_edn(&example_edn).expect("bundled story example graph failed to parse");

    let training = graph.training.as_mut().expect("bundled example should carry a training config");
    training.data_source.path_or_uri = gui_dir.join("examples/story.txt").to_string_lossy().to_string();

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");

    let checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();
    assert!(!orchestrator.has_checkpoint(&graph.graph_id), "checkpoint should not exist before training");

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone())).expect("training the story example failed");
    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should have saved a checkpoint");

    // The real vocabulary width, read directly off the bundled graph's
    // next-word head (`proj`'s `out_features`) — used below to pick out the
    // real output tensor unambiguously, rather than guessing by shape.
    let vocab_size = graph
        .nodes
        .iter()
        .find(|n| n.id == "proj")
        .and_then(|n| n.hyperparams.get("out_features"))
        .and_then(|v| v.as_u64())
        .expect("bundled graph should have a `proj` node with out_features") as usize;

    // Re-derive the exact same real (context-window, next-token) batches
    // training consumed — deterministic given the unchanged corpus + config.
    let mut data = rt
        .block_on(brainbuilder_core::data::source::load_dataset(graph.training.as_ref().unwrap()))
        .expect("failed to reload the real sequence dataset for evaluation");

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

        // `forward` returns every named intermediate tensor in unspecified
        // order (echoed inputs, weights, and real op outputs alike). Unlike
        // the tabular case `interpret.rs::predict_flat`'s "smallest tensor
        // divisible by the row count" convention assumes, the echoed `ids`
        // input here (width = seq_len = 8) is *narrower* than the real
        // output (width = vocab_size = 28) — so picking the smallest match
        // would silently grab the input tokens instead of the logits. Since
        // this test already knows the real vocab_size, match the exact
        // per-row width instead.
        let mut logits: Option<Vec<f32>> = None;
        for t in &tensors {
            let (_, values) = brainbuilder_core::interop::dlpack_support::tensor_to_vec_f32(t)
                .expect("output tensor should be float32");
            if values.len() == num_rows * vocab_size {
                logits = Some(values);
                break;
            }
        }
        let logits = logits.expect("predict should return an output tensor shaped (rows, vocab_size)");

        for row in 0..num_rows {
            let row_logits = &logits[row * vocab_size..(row + 1) * vocab_size];
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

    assert!(total > 0, "the bundled story corpus should yield at least one training window");
    let accuracy = correct as f32 / total as f32;
    assert!(
        accuracy >= 0.85,
        "the attention-based story-teller should have genuinely learned to predict the next word from \
         real context (chance ~= 1/28 = 0.036; a context-blind bigram can't solve this corpus at all), \
         got accuracy={accuracy} ({correct}/{total})"
    );
    println!("story-teller next-word accuracy: {accuracy} ({correct}/{total})");

    std::fs::remove_file(&checkpoint_path).ok();
}

fn column_f32(batch: &arrow::record_batch::RecordBatch, idx: usize) -> Vec<f32> {
    use arrow::array::Float32Array;
    batch
        .column(idx)
        .as_any()
        .downcast_ref::<Float32Array>()
        .expect("sequence dataset columns are float32")
        .values()
        .to_vec()
}

/// Drops the last column (the next-token target, by this codebase's
/// established convention) — `predict` expects exactly the context-window
/// columns the model was trained on, not the target it was trained to
/// predict (see `interpret.rs::drop_last_column`, duplicated here since it's
/// private to that module and this is a separate integration-test crate).
fn drop_last_column(batch: &arrow::record_batch::RecordBatch) -> arrow::record_batch::RecordBatch {
    let num_cols = batch.num_columns();
    let schema = batch.schema();
    let fields: Vec<arrow::datatypes::Field> =
        schema.fields().iter().take(num_cols - 1).map(|f| f.as_ref().clone()).collect();
    let columns: Vec<arrow::array::ArrayRef> = batch.columns()[..num_cols - 1].to_vec();
    arrow::record_batch::RecordBatch::try_new(std::sync::Arc::new(arrow::datatypes::Schema::new(fields)), columns)
        .expect("dropping the target column should always succeed")
}
