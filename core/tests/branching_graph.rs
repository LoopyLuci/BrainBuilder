// Regression test for the edge-wiring/port-namespacing bug fixed in
// scheduler.rs's `compile()` (topological_order + per-node-namespaced keys):
// every previous multi-node test used a strictly linear chain. This graph is
// a real diamond — one node fans out to two independent branches that merge
// back together — and all three `linear` nodes declare a parameter port
// named `weight`. Before the fix, `compile()` used the bare descriptor port
// name as the cross-op binding key, so all three "weight"s would collide in
// one flat map; `graph.edges` was never even consulted to know the fan-out
// existed. Ignored by default (needs `torch`).
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};
use brainbuilder_core::orchestrator::Orchestrator;

fn node(id: &str, component: &str, inputs: &[&str], outputs: &[&str], hp: serde_json::Value) -> BBIRNode {
    BBIRNode {
        id: id.to_string(),
        component: component.to_string(),
        label: None,
        hyperparams: hp,
        ports: PortInfo {
            input_ports: inputs.iter().map(|s| s.to_string()).collect(),
            output_ports: outputs.iter().map(|s| s.to_string()).collect(),
        },
        position: None,
    }
}

fn edge(from_node: &str, from_port: &str, to_node: &str, to_port: &str) -> BBIREdge {
    BBIREdge {
        from_node: from_node.to_string(),
        from_port: from_port.to_string(),
        to_node: to_node.to_string(),
        to_port: to_port.to_string(),
    }
}

#[test]
#[ignore]
fn diamond_shaped_graph_with_colliding_port_names_trains_correctly() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Seed weight init so this run is deterministic (the worker calls
    // torch.manual_seed on startup when BRAINBUILDER_SEED is set — see
    // _bb_worker.py::_apply_seed). This removes the run-to-run convergence
    // variance that used to make this test flaky on shared CI runners, without
    // weakening what it checks. Set before `Orchestrator::new`, which spawns
    // the worker that reads it.
    std::env::set_var("BRAINBUILDER_SEED", "1234");

    let csv_dir = std::env::temp_dir().join("bb_branching_graph");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..30 {
        let x1 = (i as f32 - 15.0) * 0.2;
        let x2 = (i as f32 - 10.0) * 0.15;
        let y = 1.5 * x1 - 0.5 * x2;
        csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    //      lin (in=2, out=2)
    //     /                 \
    // branch_a (2->1)   branch_b (2->1)
    //     \                 /
    //           merge (add)
    //
    // `lin` outputs a full 2-D signal (not a 1-D bottleneck) so each branch
    // has its own independent 2-D-to-1-D linear map — a well-conditioned
    // optimization landscape. (An earlier version routed lin through a
    // single scalar into both branches: mathematically fine but a
    // product-of-scalars saddle that gradient descent gets stuck near —
    // a bad test design, not evidence of anything broken in the engine.)
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "branching-graph-test".to_string(),
        name: "branching-graph-test".to_string(),
        nodes: vec![
            node("lin", "linear", &["input", "weight"], &["output"], serde_json::json!({"in_features": 2, "out_features": 2})),
            node("branch_a", "linear", &["input", "weight"], &["output"], serde_json::json!({"in_features": 2, "out_features": 1})),
            node("branch_b", "linear", &["input", "weight"], &["output"], serde_json::json!({"in_features": 2, "out_features": 1})),
            node("merge", "add", &["a", "b"], &["output"], serde_json::json!({})),
        ],
        edges: vec![
            edge("lin", "output", "branch_a", "input"),
            edge("lin", "output", "branch_b", "input"),
            edge("branch_a", "output", "merge", "a"),
            edge("branch_b", "output", "merge", "b"),
        ],
        training: Some(TrainingConfig {
            loss: "mse".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            // Random weight init (no seeding is plumbed through
            // `PythonBridge::random_tensor` yet — a real, separate gap) means
            // a run can land near a slow-converging region; enough real
            // steps (many epochs x small batch_size) makes convergence
            // reliable regardless of which random init this run happened to
            // draw, rather than tuning to whatever one lucky run needed.
            hyperparams: serde_json::json!({"lr": 0.05, "epochs": 500}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
                batch_size: 5,
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

    // The metrics broadcast channel has a fixed 200-slot capacity (see
    // data/metrics.rs) — fine for the GUI's live subscriber, which drains
    // continuously in real time, but this test's ~1800 real training steps
    // would blow past that and start silently dropping (`Lagged`) points if
    // collected only *after* `execute_graph` finishes. Draining concurrently
    // via a background task, the same way the GUI's own subscriber does,
    // avoids that entirely regardless of step count.
    let points = std::sync::Arc::new(std::sync::Mutex::new(Vec::<MetricPoint>::new()));
    let points_for_drain = points.clone();
    let mut metrics_rx = subscribe_metrics();

    let rt = tokio::runtime::Runtime::new().unwrap();
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
        orchestrator.execute_graph(graph.clone()).await.expect("training the diamond graph failed");
        drain.abort();
    });

    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should have saved a checkpoint");

    let points = points.lock().unwrap();
    let first_loss = points.first().unwrap().loss;
    let last_loss = points.last().unwrap().loss;
    // Weight init is now seeded (BRAINBUILDER_SEED, set at the top), so this
    // run is deterministic — the historical run-to-run convergence variance
    // that made this test flaky is gone. The bar still targets what this test
    // is really about: it must clearly learn *something* (rules out the
    // original bug this regression-tests, where loss was bit-for-bit identical
    // every step because epochs silently never re-iterated the data), while
    // the namespacing check below is the core regression guard.
    assert!(
        last_loss < first_loss * 0.9,
        "expected the diamond graph's loss to measurably drop, not stay flat: first={first_loss}, last={last_loss}"
    );

    // The real regression check: three independently-trained `weight`
    // tensors must exist under distinct, node-namespaced keys, not one
    // shared/overwritten value. Loading the checkpoint's raw state dict
    // (rather than going through the graph again) proves the *persisted*
    // keys are actually separate, not just separate in memory.
    let bridge = brainbuilder_core::interop::python::PythonBridge::new(std::sync::Arc::new(
        brainbuilder_core::interop::arena::SharedArena::new(),
    ))
    .unwrap();
    let weights = bridge.load_state_dict(&checkpoint_path).expect("failed to load checkpoint");
    for key in ["lin:weight", "branch_a:weight", "branch_b:weight"] {
        assert!(weights.contains_key(key), "checkpoint missing expected namespaced key `{key}`");
    }

    fn read_all(t: &brainbuilder_core::Tensor) -> Vec<f32> {
        unsafe {
            let dl = &(*t.0).dl_tensor;
            let mut n = 1usize;
            for i in 0..dl.ndim {
                n *= *dl.shape.offset(i as isize) as usize;
            }
            std::slice::from_raw_parts(dl.data as *const f32, n).to_vec()
        }
    }
    let branch_a_w = read_all(weights.get("branch_a:weight").unwrap());
    let branch_b_w = read_all(weights.get("branch_b:weight").unwrap());
    assert_ne!(
        branch_a_w, branch_b_w,
        "branch_a and branch_b are independent parameters (different random init + independent \
         gradients) — identical values would mean they were silently aliased/collided"
    );
}
