// Proves the new `image_folder` data source trains a real classifier end to
// end, through the exact `Orchestrator::execute_graph` path the GUI's Tauri
// command calls — the same path every other dataset type uses. It generates a
// genuine two-class folder of solid-color PNGs on disk (red vs. blue),
// decodes them through `data::vision`, and trains a `linear` classifier with
// `cross_entropy` on the flattened pixel features. Because red and blue are
// perfectly linearly separable in raw RGB, the loss must drop substantially —
// that's the checkable signal that the whole chain (decode → resize →
// normalize → column-stack → class-index target → forward/backward) is real.
//
// Ignored by default: needs `torch` on the worker's PYTHONPATH, like every
// other end-to-end training test here.
use brainbuilder_core::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};
use brainbuilder_core::orchestrator::Orchestrator;
use image::{Rgb, RgbImage};
use std::path::Path;

fn write_png(path: &Path, color: [u8; 3]) {
    // Slight per-file jitter so every image isn't byte-identical (a degenerate
    // dataset) while staying firmly in its color's region of RGB space.
    let mut img = RgbImage::new(8, 8);
    for (i, pixel) in img.pixels_mut().enumerate() {
        let j = (i % 7) as i16 - 3;
        let c = |v: u8| (v as i16 + j).clamp(0, 255) as u8;
        *pixel = Rgb([c(color[0]), c(color[1]), c(color[2])]);
    }
    img.save(path).expect("write test png");
}

#[test]
#[ignore]
fn linear_classifier_trains_on_a_real_image_folder() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));
    // Seed weight init for a deterministic, non-flaky run on CI (see
    // _bb_worker.py::_apply_seed). The data is trivially separable, so it
    // converges either way, but a fixed seed removes any run-to-run variance.
    std::env::set_var("BRAINBUILDER_SEED", "7");

    // Build a real folder-of-classes dataset on disk.
    let root = std::env::temp_dir().join(format!("bb_image_folder_train_{}", uuid::Uuid::new_v4()));
    let red = root.join("red");
    let blue = root.join("blue");
    std::fs::create_dir_all(&red).unwrap();
    std::fs::create_dir_all(&blue).unwrap();
    for i in 0..10 {
        write_png(&red.join(format!("{i}.png")), [220, 20, 20]);
        write_png(&blue.join(format!("{i}.png")), [20, 20, 220]);
    }

    // 16x16 RGB => 16*16*3 = 768 input features; 2 classes => 2 output logits.
    let image_size = 16usize;
    let in_features = image_size * image_size * 3;
    let num_classes = 2;

    let graph = BBIRGraph {
        schema_version: 1,
        graph_id: format!("image-folder-classifier-{}", uuid::Uuid::new_v4()),
        name: "image-folder-classifier-test".to_string(),
        nodes: vec![BBIRNode {
            id: "cls".to_string(),
            component: "linear".to_string(),
            label: None,
            hyperparams: serde_json::json!({"in_features": in_features, "out_features": num_classes}),
            ports: PortInfo {
                input_ports: vec!["input".to_string(), "weight".to_string()],
                output_ports: vec!["output".to_string()],
            },
            position: None,
        }],
        edges: Vec::<BBIREdge>::new(),
        training: Some(TrainingConfig {
            loss: "cross_entropy".to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.01, "epochs": 30}),
            data_source: DataSourceConfig {
                source_type: "image_folder".to_string(),
                path_or_uri: root.to_string_lossy().to_string(),
                batch_size: 20,
                preprocessing: Vec::new(),
                sequence_length: None,
                vocab_size: None,
                image_size: Some(image_size),
                grayscale: Some(false),
                text_column: None,
                label_column: None,
            },
            reproducibility: None,
        }),
    };

    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let checkpoint_path = components_dir
        .parent()
        .unwrap()
        .join("checkpoints")
        .join(format!("{}.pt", graph.graph_id));
    std::fs::remove_file(&checkpoint_path).ok();

    let mut metrics_rx = subscribe_metrics();

    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(orchestrator.execute_graph(graph.clone()))
        .expect("training an image-folder classifier failed");

    assert!(orchestrator.has_checkpoint(&graph.graph_id), "training should have saved a checkpoint");

    let mut points: Vec<MetricPoint> = Vec::new();
    while let Ok(p) = metrics_rx.try_recv() {
        points.push(p);
    }
    assert!(points.len() >= 4, "expected multiple training steps' worth of metrics, got {}", points.len());

    let first_loss = points.first().unwrap().loss;
    let last_loss = points.last().unwrap().loss;
    assert!(
        last_loss < first_loss * 0.5,
        "expected classification loss to drop substantially on linearly-separable red-vs-blue images: \
         first={first_loss}, last={last_loss}"
    );

    std::fs::remove_dir_all(&root).ok();
}
