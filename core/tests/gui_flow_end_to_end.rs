// Exercises the exact user journey Phase 0 promises — load the bundled
// first-run example, train it, get a checkpoint, run Predict — through the
// same `Orchestrator` API the GUI's Tauri commands call. This can't be a
// literal GUI click-test (no window-capture tooling available here), but it
// runs the identical Rust code path `execute_graph`/`predict` invoke, against
// the real bundled example.bbir.edn + example.csv + linear.py, with real
// PyTorch. Ignored by default — run with `cargo test -- --ignored`.
use brainbuilder_core::bbir::BBIRGraph;
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn bundled_first_run_example_trains_and_predicts() {
    // All paths below are absolute (built from CARGO_MANIFEST_DIR), so this
    // test doesn't depend on — or need to touch — the process's cwd, unlike
    // the real app's `../components`/`examples/...` relative-path resolution
    // convention (main.rs). `Orchestrator::new` derives checkpoints_dir and
    // the provenance DB from `components_dir`'s parent, i.e. the real repo's
    // `checkpoints/`/`provenance.sqlite3` — same as a real run.
    let gui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui");
    let components_dir = gui_dir.join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let example_edn = std::fs::read_to_string(gui_dir.join("examples/first_run.bbir.edn"))
        .expect("bundled example graph missing");
    let graph = BBIRGraph::from_edn(&example_edn).expect("bundled example graph failed to parse");

    // execute_graph resolves data_source.path_or_uri relative to cwd, which
    // we just changed to `scratch` — patch it to the real bundled CSV's
    // absolute path so training can actually find the data.
    let mut graph = graph;
    if let Some(training) = graph.training.as_mut() {
        training.data_source.path_or_uri = gui_dir
            .join("examples/first_run.csv")
            .to_string_lossy()
            .to_string();
    }

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");

    // Idempotent: a prior run of this same test (or a manual run of the real
    // app against the bundled example) may have already left a checkpoint
    // for this graph_id — clear it so "training saves a checkpoint" is
    // actually exercised, not trivially true from a stale file.
    let checkpoint_path = components_dir
        .parent()
        .unwrap()
        .join("checkpoints")
        .join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();
    assert!(
        !orchestrator.has_checkpoint(&graph.graph_id),
        "checkpoint should not exist before training"
    );

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone()))
        .expect("training the bundled example failed");

    assert!(
        orchestrator.has_checkpoint(&graph.graph_id),
        "training should have saved a checkpoint"
    );

    let rt2 = tokio::runtime::Runtime::new().unwrap();
    let batch = rt2
        .block_on(brainbuilder_core::data::source::load_batch(
            &graph.training.as_ref().unwrap().data_source.path_or_uri,
            5,
        ))
        .expect("failed to load a batch for prediction");

    let predictions = orchestrator
        .predict(graph, batch)
        .expect("predict failed even though a checkpoint exists");

    assert!(!predictions.is_empty(), "predict should return at least one output tensor");
}
