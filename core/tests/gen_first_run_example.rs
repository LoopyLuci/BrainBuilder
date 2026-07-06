// Not a real test — a one-off generator for gui/examples/first_run.bbir.edn,
// run manually (`cargo test --test gen_first_run_example -- --ignored
// --nocapture`) whenever the bundled first-run example needs regenerating.
// Writing it through the real `BBIRGraph::to_edn` serializer (rather than
// hand-typing EDN) guarantees the bundled file matches whatever format the
// app's own save/load path actually produces.
use brainbuilder_core::bbir::{
    BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, NodePosition, PortInfo, TrainingConfig,
};

#[test]
#[ignore]
fn generate_first_run_example() {
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "00000000-0000-0000-0000-000000000001".to_string(),
        name: "first-run-example".to_string(),
        nodes: vec![BBIRNode {
            id: "n1".to_string(),
            component: "scale".to_string(),
            label: Some("Scale".to_string()),
            hyperparams: serde_json::json!({}),
            ports: PortInfo {
                input_ports: vec!["x".to_string(), "w".to_string()],
                output_ports: vec!["y".to_string()],
            },
            position: Some(NodePosition { x: 250.0, y: 150.0 }),
        }],
        edges: Vec::<BBIREdge>::new(),
        training: Some(TrainingConfig {
            loss: "mse".to_string(),
            optimizer: "sgd".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 30}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                // Relative to the Tauri backend's working directory at
                // runtime (cwd = gui/, same convention `components_dir` uses
                // in main.rs) — resolves to gui/examples/first_run.csv.
                path_or_uri: "examples/first_run.csv".to_string(),
                batch_size: 20,
                preprocessing: Vec::new(),
                sequence_length: None,
                vocab_size: None,
            },
            reproducibility: None,
        }),
    };

    let edn = graph.to_edn().expect("serialize example graph");
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui/examples/first_run.bbir.edn");
    std::fs::write(&out, edn).expect("write example graph");
    println!("wrote {}", out.display());
}
