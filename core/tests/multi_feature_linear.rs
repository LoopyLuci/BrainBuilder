// Proves the Arrow-bridge column-stacking fix: a `linear` graph with TWO
// feature columns (plus a target column) trains end to end from a real CSV,
// where previously only single-column/elementwise components like `scale`
// worked. y = 2*x1 + 3*x2 is a real, checkable linear relationship.
use brainbuilder_core::bbir::{
    BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig,
};
use brainbuilder_core::orchestrator::Orchestrator;

#[test]
#[ignore]
fn linear_trains_on_a_real_multi_feature_csv() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    let csv_dir = std::env::temp_dir().join("bb_multi_feature_linear");
    std::fs::create_dir_all(&csv_dir).unwrap();
    let csv_path = csv_dir.join("data.csv");
    let mut csv = String::from("x1,x2,y\n");
    for i in 0..20 {
        let x1 = (i as f32 - 10.0) * 0.3;
        let x2 = (i as f32 - 5.0) * 0.2;
        let y = 2.0 * x1 + 3.0 * x2;
        csv.push_str(&format!("{x1},{x2},{y}\n"));
    }
    std::fs::write(&csv_path, csv).unwrap();

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: "multi-feature-linear-test".to_string(),
        name: "multi-feature-linear-test".to_string(),
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
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 50}),
            data_source: DataSourceConfig {
                source_type: "file".to_string(),
                path_or_uri: csv_path.to_string_lossy().to_string(),
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
    rt.block_on(orchestrator.execute_graph(graph.clone()))
        .expect("training a multi-feature linear graph failed");

    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should save a checkpoint");
}
