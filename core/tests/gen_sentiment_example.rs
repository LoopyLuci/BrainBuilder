// Not a real test — a one-off generator for gui/examples/sentiment.bbir.edn,
// run manually (`cargo test --test gen_sentiment_example -- --ignored
// --nocapture`) whenever the bundled sentiment example needs regenerating.
// A real bag-of-words text classifier (linear -> gelu -> linear, trained
// against gui/examples/sentiment.csv via the text_column data source) — the
// second bundled example after first_run, and meaningfully different: it
// actually classifies something (positive/negative sentiment) instead of
// fitting a single scalar.
//
// `in_features` on the first linear node must exactly equal the realized
// bag-of-words vocabulary width, which depends on the real corpus (see
// `data::tabular_text::build_bag_of_words`) — rather than hand-counting
// distinct words, this generator computes it by calling the exact same real
// code the training path calls (`source::read_text_label_rows` +
// `tabular_text::build_bag_of_words`), so the generated graph can never drift
// out of sync with the dataset.
use brainbuilder_core::bbir::{
    BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, NodePosition, PortInfo, TrainingConfig,
};
use brainbuilder_core::data::{source, tabular_text};

#[test]
#[ignore]
fn generate_sentiment_example() {
    let gui_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../gui");
    let csv_path = gui_dir.join("examples/sentiment.csv");

    // Comfortably above the corpus's real distinct-word count, so the
    // frequency cap in `Vocab::build` never actually kicks in and every word
    // gets its own column — keeps the generated graph's shape a direct,
    // unsurprising function of the dataset.
    let vocab_size = 500usize;

    let probe_cfg = DataSourceConfig {
        source_type: "text_column".to_string(),
        path_or_uri: csv_path.to_string_lossy().to_string(),
        batch_size: 8,
        preprocessing: Vec::new(),
        sequence_length: None,
        vocab_size: Some(vocab_size),
        image_size: None,
        grayscale: None,
        text_column: Some("text".to_string()),
        label_column: Some("label".to_string()),
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let rows = rt
        .block_on(source::read_text_label_rows(&probe_cfg))
        .expect("failed to read sentiment.csv");
    let dataset = tabular_text::build_bag_of_words(&rows, vocab_size).expect("failed to build vocab");
    let feature_count = dataset.feature_count;
    let num_classes = dataset.class_names.len();
    println!("real bag-of-words width: {feature_count}, classes: {:?}", dataset.class_names);

    let hidden = 32;

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "00000000-0000-0000-0000-000000000002".to_string(),
        name: "sentiment-classifier-example".to_string(),
        nodes: vec![
            BBIRNode {
                id: "n1".to_string(),
                component: "linear".to_string(),
                label: Some("Linear 1".to_string()),
                hyperparams: serde_json::json!({"in_features": feature_count, "out_features": hidden}),
                ports: PortInfo {
                    input_ports: vec!["input".to_string(), "weight".to_string()],
                    output_ports: vec!["output".to_string()],
                },
                position: Some(NodePosition { x: 80.0, y: 150.0 }),
            },
            BBIRNode {
                id: "n2".to_string(),
                component: "gelu".to_string(),
                label: Some("GELU".to_string()),
                hyperparams: serde_json::json!({}),
                ports: PortInfo {
                    input_ports: vec!["input".to_string()],
                    output_ports: vec!["output".to_string()],
                },
                position: Some(NodePosition { x: 340.0, y: 150.0 }),
            },
            BBIRNode {
                id: "n3".to_string(),
                component: "linear".to_string(),
                label: Some("Linear 2".to_string()),
                hyperparams: serde_json::json!({"in_features": hidden, "out_features": num_classes}),
                ports: PortInfo {
                    input_ports: vec!["input".to_string(), "weight".to_string()],
                    output_ports: vec!["output".to_string()],
                },
                position: Some(NodePosition { x: 600.0, y: 150.0 }),
            },
        ],
        edges: vec![
            BBIREdge {
                from_node: "n1".to_string(),
                from_port: "output".to_string(),
                to_node: "n2".to_string(),
                to_port: "input".to_string(),
            },
            BBIREdge {
                from_node: "n2".to_string(),
                from_port: "output".to_string(),
                to_node: "n3".to_string(),
                to_port: "input".to_string(),
            },
        ],
        training: Some(TrainingConfig {
            loss: "cross_entropy".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 120}),
            data_source: DataSourceConfig {
                // Relative to the Tauri backend's working directory at
                // runtime (cwd = gui/), same convention `first_run` uses —
                // resolves to gui/examples/sentiment.csv.
                path_or_uri: "examples/sentiment.csv".to_string(),
                ..probe_cfg
            },
            reproducibility: None,
        }),
    };

    let edn = graph.to_edn().expect("serialize sentiment example graph");
    let out = gui_dir.join("examples/sentiment.bbir.edn");
    std::fs::write(&out, edn).expect("write example graph");
    println!("wrote {}", out.display());
}
