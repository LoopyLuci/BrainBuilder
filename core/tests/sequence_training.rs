// Closes a real gap: the transformer/attention/embedding components proven
// in transformer_components.rs work at the engine level, but until now there
// was no path from an actual dataset file to the (batch, seq) integer-id
// tensor an `embedding` node needs — the GUI's dataset loader only produced
// 2-D (batch, features) tabular batches. This test proves the new
// `data::text` tokenizer + `text_sequence` data source actually feeds a
// real multi-node graph through the exact `Orchestrator::execute_graph` path
// the GUI's Tauri command calls, and that real next-token-prediction
// training measurably reduces loss. Ignored by default (needs `torch`).
//
// It also exercises (for the first time in this test suite) a genuinely
// multi-node, edge-connected graph through `compile()` — every previous
// end-to-end test used exactly one node. `embedding` and `linear` both
// declare a parameter port named `weight`; without `compile()`'s real
// edge-based, per-node-namespaced wiring (see scheduler.rs's
// `topological_order`/`namespaced` helpers), the two would collide in the
// old flat-port-name intermediate map and this graph would silently train
// the wrong thing (or crash on a shape mismatch) instead of erroring loudly.
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};
use brainbuilder_core::orchestrator::Orchestrator;

fn node(id: &str, component: &str, inputs: &[&str], outputs: &[&str], hyperparams: serde_json::Value) -> BBIRNode {
    BBIRNode {
        id: id.to_string(),
        component: component.to_string(),
        label: None,
        hyperparams,
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
fn tokenized_text_trains_a_real_embedding_to_linear_graph_and_loss_drops() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    // A perfectly regular 4-token cycle repeated many times: the true
    // next-token function is trivial (predict position i mod 4) but real —
    // a model that isn't actually learning would not converge on it.
    let text_dir = std::env::temp_dir().join("bb_sequence_training");
    std::fs::create_dir_all(&text_dir).unwrap();
    let text_path = text_dir.join("corpus.txt");
    let corpus = "alpha beta gamma delta ".repeat(60);
    std::fs::write(&text_path, &corpus).unwrap();

    let vocab_size = 8; // 4 real words + <unk> + headroom
    let embedding_dim = 8;
    let seq_len = 4;

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "sequence-training-test".to_string(),
        name: "sequence-training-test".to_string(),
        nodes: vec![
            node(
                "embed",
                "embedding",
                &["ids", "weight"],
                &["output"],
                serde_json::json!({"vocab_size": vocab_size, "embedding_dim": embedding_dim}),
            ),
            node("pool", "select_last", &["input"], &["output"], serde_json::json!({})),
            node(
                "proj",
                "linear",
                &["input", "weight"],
                &["output"],
                serde_json::json!({"in_features": embedding_dim, "out_features": vocab_size}),
            ),
        ],
        edges: vec![
            edge("embed", "output", "pool", "input"),
            edge("pool", "output", "proj", "input"),
        ],
        training: Some(TrainingConfig {
            loss: "cross_entropy".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.05, "epochs": 8}),
            data_source: DataSourceConfig {
                source_type: "text_sequence".to_string(),
                path_or_uri: text_path.to_string_lossy().to_string(),
                batch_size: 32,
                preprocessing: Vec::new(),
                sequence_length: Some(seq_len),
                vocab_size: Some(vocab_size),
            },
            reproducibility: None,
        }),
    };

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let checkpoint_path = components_dir.parent().unwrap().join("checkpoints").join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();

    let mut metrics_rx = subscribe_metrics();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone()))
        .expect("training the tokenized-text sequence graph failed");

    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should have saved a checkpoint");

    let mut points: Vec<MetricPoint> = Vec::new();
    while let Ok(p) = metrics_rx.try_recv() {
        points.push(p);
    }
    assert!(points.len() >= 4, "expected multiple training steps' worth of metrics, got {}", points.len());

    let first_loss = points.first().unwrap().loss;
    let last_loss = points.last().unwrap().loss;
    assert!(
        last_loss < first_loss * 0.7,
        "expected next-token-prediction loss to drop substantially on a trivially learnable \
         cyclic corpus: first={first_loss}, last={last_loss}"
    );
}
