use brainbuilder_core::bbir::{
    BBIRGraph, BBIRNode, DataSourceConfig, NodePosition, PortInfo, TrainingConfig,
};

#[test]
fn edn_roundtrip_preserves_graph() {
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "g1".to_string(),
        name: "demo".to_string(),
        nodes: vec![],
        edges: vec![],
        training: None,
    };

    let edn = graph.to_edn().expect("serialize to edn");
    let parsed = BBIRGraph::from_edn(&edn).expect("parse edn back");

    assert_eq!(parsed.graph_id, graph.graph_id);
    assert_eq!(parsed.name, graph.name);
    assert!(parsed.nodes.is_empty());
    assert!(parsed.training.is_none());
}

#[test]
fn edn_roundtrip_preserves_node_position() {
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "g2".to_string(),
        name: "with-node".to_string(),
        nodes: vec![BBIRNode {
            id: "n1".to_string(),
            component: "linear".to_string(),
            label: Some("Linear 1".to_string()),
            hyperparams: serde_json::json!({}),
            ports: PortInfo { input_ports: vec!["input".into()], output_ports: vec!["output".into()] },
            position: Some(NodePosition { x: 120.5, y: -40.0 }),
        }],
        edges: vec![],
        training: None,
    };

    let edn = graph.to_edn().expect("serialize to edn");
    let parsed = BBIRGraph::from_edn(&edn).expect("parse edn back");

    let node = &parsed.nodes[0];
    assert_eq!(node.label.as_deref(), Some("Linear 1"));
    let pos = node.position.expect("position should round-trip");
    assert_eq!(pos.x, 120.5);
    assert_eq!(pos.y, -40.0);
}

/// Regression test: `edn_rs::json_to_edn` (the crate's own helper) mangles
/// numeric object values — `{"lr": 0.01, "epochs": 30}` came out as the
/// malformed `{:epochs30,:lr0.01}` (key and value glued together). This
/// wasn't caught by the other roundtrip tests because none of them exercised
/// `hyperparams` with actual numeric content. Verified fixed by our own
/// hand-written `interop::edn_value::json_to_edn`.
#[test]
fn edn_roundtrip_preserves_numeric_hyperparams() {
    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "g3".to_string(),
        name: "with-hyperparams".to_string(),
        nodes: vec![],
        edges: vec![],
        training: Some(TrainingConfig {
            loss: "mse".to_string(),
            optimizer: "sgd".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 30}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: "data.csv".to_string(),
                batch_size: 16,
                preprocessing: vec![],
                sequence_length: None,
                vocab_size: None,
            },
            reproducibility: None,
        }),
    };

    let edn = graph.to_edn().expect("serialize to edn");
    let parsed = BBIRGraph::from_edn(&edn).expect("parse edn back");

    let hyperparams = parsed.training.expect("training config").hyperparams;
    assert_eq!(hyperparams.get("lr").and_then(|v| v.as_f64()), Some(0.01));
    assert_eq!(hyperparams.get("epochs").and_then(|v| v.as_i64()), Some(30));
}

/// Real backward-compatibility check: the app's own bundled first-run
/// example (`gui/examples/first_run.bbir.edn`) was saved before
/// `schema_version` existed at all — no `:schema-version` key anywhere in
/// the file. A person's real graphs saved before this change look exactly
/// like this. `BBIRGraph::from_edn` must still parse it (defaulting to
/// version 1, the only format that predates the field) rather than erroring
/// out or panicking the moment an older save is opened in a newer build.
#[test]
fn loads_a_real_pre_versioning_saved_graph_and_defaults_its_schema_version() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui/examples/first_run.bbir.edn");
    let edn = std::fs::read_to_string(&path).expect("bundled first-run example must exist");
    assert!(!edn.contains("schema-version"), "fixture should genuinely predate schema_version");

    let graph = BBIRGraph::from_edn(&edn).expect("a pre-versioning saved graph must still load");
    assert_eq!(graph.schema_version, 1);
    assert_eq!(graph.name, "first-run-example");
    assert_eq!(graph.nodes.len(), 1);
}
