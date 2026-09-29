// Proves `standard_trainer::StandardTrainer::fit`'s early-stopping path runs
// for real. The plateauing scenario deliberately trains with `lr: 0.0`
// rather than relying on a real model's natural convergence: an earlier
// version of this test used a bare single-node `linear` graph's genuinely
// slow (but not truly flat) loss curve, expecting it to plateau within a
// modest epoch budget — it never did (each epoch's improvement kept
// shrinking geometrically but stayed above the no-improvement threshold for
// the whole 50-epoch budget), so the test just measured how slowly that
// graph converges, not whether early stopping works. `lr: 0.0` sidesteps
// that entirely: real forward/backward/optimizer calls still happen every
// step, but the loss is now provably identical epoch to epoch (no weight
// update occurs), which drives early stopping's own logic deterministically
// regardless of convergence dynamics, environment, or a real model's own
// unrelated quirks.
//
// Both scenarios below run inside one `#[test]` function, sequentially, on
// purpose: `data::metrics`'s broadcast channel is a single process-global
// static with no per-graph filtering, so two `#[test]` fns each training a
// graph would run concurrently (Rust's default parallel test harness) and
// contaminate each other's collected points — caught by an early version of
// this file where a "patience unset" run observed an extra epoch that
// actually belonged to the other, concurrently-running test.
// Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};
use brainbuilder_core::orchestrator::Orchestrator;

fn single_linear_graph(graph_id: &str, csv_path: &std::path::Path, hyperparams: serde_json::Value) -> BBIRGraph {
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
            hyperparams,
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                // One batch per epoch (batch_size == row count), so "epoch"
                // and "patience-counted step" are the same unit — keeps the
                // assertions simple to reason about.
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

/// Trains `graph` to completion, collecting every metric point it publishes
/// (draining concurrently, same reasoning as `branching_graph.rs`'s pattern:
/// the broadcast channel's fixed capacity could otherwise drop points).
fn train_and_collect(orchestrator: &Orchestrator, graph: &BBIRGraph, rt: &tokio::runtime::Runtime) -> Vec<MetricPoint> {
    let points = std::sync::Arc::new(std::sync::Mutex::new(Vec::<MetricPoint>::new()));
    let points_for_drain = points.clone();
    let mut metrics_rx = subscribe_metrics();

    rt.block_on(async {
        let drain = tokio::spawn(async move {
            loop {
                match metrics_rx.recv().await {
                    Ok(p) => points_for_drain.lock().unwrap().push(p),
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
        orchestrator.execute_graph(graph.clone()).await.expect("training failed");
        drain.abort();
    });

    // Not `Arc::try_unwrap`: `drain.abort()` only *requests* cancellation —
    // the spawned task's `Arc` clone isn't guaranteed dropped the instant
    // `abort()` returns, so unwrapping here raced it and occasionally failed
    // even though every point had already been pushed. Cloning the (already
    // fully populated, by this point) Vec out from behind the lock sidesteps
    // that race entirely.
    let points = points.lock().unwrap().clone();
    points
}

#[test]
#[ignore]
fn early_stopping_behaves_correctly() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    std::env::set_var("BRAINBUILDER_SEED", "1234");

    let csv_dir = std::env::temp_dir().join("bb_early_stopping");
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
    let rt = tokio::runtime::Runtime::new().unwrap();

    // Scenario 1: lr: 0.0, patience: 3, epochs: 20 — the loss is identical
    // every epoch (no weight update ever occurs), so this deterministically
    // stops exactly `patience` epochs after the first (best) one: epoch 0
    // sets the baseline, epochs 1-3 each fail to improve on it, and the
    // third failure (epochs_without_improvement reaching patience) breaks
    // the loop — four epochs total, regardless of machine or torch version.
    let plateauing_id = "early-stopping-test";
    let plateauing_graph =
        single_linear_graph(plateauing_id, &csv_path, serde_json::json!({"lr": 0.0, "epochs": 20, "patience": 3}));
    let checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{plateauing_id}.pt"));
    std::fs::remove_file(&checkpoint_path).ok();

    let points = train_and_collect(&orchestrator, &plateauing_graph, &rt);
    assert!(orchestrator.has_checkpoint(plateauing_id));
    assert!(!points.is_empty(), "training should have published at least one metric point");

    let last = points.last().unwrap();
    assert!(last.stopped_early, "the final metric point should be flagged stopped_early: {last:?}");

    let epochs_run = points.iter().map(|p| p.epoch).max().unwrap() + 1;
    assert_eq!(
        epochs_run, 4,
        "with a flat loss curve and patience 3, training should stop after exactly 4 epochs (1 baseline + 3 non-improving), got {epochs_run}"
    );
    std::fs::remove_file(&checkpoint_path).ok();

    // Scenario 2 (sequential, not concurrent — see file header comment):
    // patience unset (defaults to 0, "off") must run every configured
    // epoch, exactly as if this feature didn't exist — the regression guard
    // for a graph saved before `patience` existed at all.
    let off_id = "early-stopping-off-test";
    let off_graph = single_linear_graph(off_id, &csv_path, serde_json::json!({"lr": 0.01, "epochs": 5}));
    let off_checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{off_id}.pt"));
    std::fs::remove_file(&off_checkpoint_path).ok();

    let off_points = train_and_collect(&orchestrator, &off_graph, &rt);
    let off_epochs_run = off_points.iter().map(|p| p.epoch).max().unwrap() + 1;
    assert_eq!(off_epochs_run, 5, "with patience unset, all 5 configured epochs should have run");
    assert!(!off_points.last().unwrap().stopped_early, "an unset patience must never flag stopped_early");
    std::fs::remove_file(&off_checkpoint_path).ok();
}
