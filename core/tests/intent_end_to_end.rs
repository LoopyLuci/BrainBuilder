// The whole point of the Intent layer, proven end to end: a zero-knowledge
// user points at a real folder of images and picks "classify" — and gets back
// a model that actually trains and learns, with no graph editing, no
// component knowledge, no hyperparameter tuning. This generates a genuine
// two-class image folder, calls `intent::propose_model` (the exact code the
// GUI's front-door command calls), then trains the *proposed* graph through
// the exact `Orchestrator::execute_graph` path, and asserts the loss drops.
//
// Ignored by default: needs `torch` on the worker's PYTHONPATH.
use brainbuilder_core::component::registry::ComponentRegistry;
use brainbuilder_core::data::metrics::{subscribe_metrics, MetricPoint};
use brainbuilder_core::intent::{propose_model, DataSpec, IntentRequest, TaskKind};
use brainbuilder_core::orchestrator::Orchestrator;
use image::{Rgb, RgbImage};
use std::path::Path;

fn write_png(path: &Path, color: [u8; 3]) {
    let mut img = RgbImage::new(8, 8);
    for (i, pixel) in img.pixels_mut().enumerate() {
        let j = (i % 5) as i16 - 2;
        let c = |v: u8| (v as i16 + j).clamp(0, 255) as u8;
        *pixel = Rgb([c(color[0]), c(color[1]), c(color[2])]);
    }
    img.save(path).unwrap();
}

#[test]
#[ignore]
fn a_proposed_image_classifier_actually_trains() {
    let components_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
    std::env::set_var("PYTHONPATH", components_dir.join("python"));

    // Real folder-of-classes data the user "points at".
    let root = std::env::temp_dir().join(format!("bb_intent_e2e_{}", uuid::Uuid::new_v4()));
    for (class, color) in [("green", [20u8, 200, 20]), ("purple", [140, 20, 160])] {
        let d = root.join(class);
        std::fs::create_dir_all(&d).unwrap();
        for i in 0..10 {
            write_png(&d.join(format!("{i}.png")), color);
        }
    }

    // The user's entire input: "classify these images."
    let request = IntentRequest {
        task: TaskKind::Classification,
        data: DataSpec {
            source_type: "image_folder".to_string(),
            path: root.to_string_lossy().to_string(),
            image_size: Some(16),
            grayscale: Some(false),
            text_column: None,
            label_column: None,
            vocab_size: None,
        },
    };

    let mut registry = ComponentRegistry::new();
    registry.load_from_dir(&components_dir).expect("components must load");

    let rt = tokio::runtime::Runtime::new().unwrap();
    let proposal = rt.block_on(propose_model(&request, &registry)).expect("Intent layer should propose a model");

    // Sanity: the proposal really is sized to the data.
    assert_eq!(proposal.class_names, vec!["green".to_string(), "purple".to_string()]);
    assert_eq!(proposal.num_outputs, 2);

    // Now train exactly what was proposed — no edits.
    let orchestrator = Orchestrator::new(&components_dir).expect("orchestrator init failed");
    let checkpoint = components_dir
        .parent()
        .unwrap()
        .join("checkpoints")
        .join(format!("{}.pt", proposal.graph.graph_id));
    std::fs::remove_file(&checkpoint).ok();

    let mut metrics_rx = subscribe_metrics();
    rt.block_on(orchestrator.execute_graph(proposal.graph.clone()))
        .expect("training the proposed graph failed");

    let mut points: Vec<MetricPoint> = Vec::new();
    while let Ok(p) = metrics_rx.try_recv() {
        points.push(p);
    }
    assert!(points.len() >= 4, "expected multiple training steps, got {}", points.len());
    let first = points.first().unwrap().loss;
    let last = points.last().unwrap().loss;
    assert!(
        last < first * 0.6,
        "a proposed classifier should learn on separable data: first={first}, last={last}"
    );

    std::fs::remove_dir_all(&root).ok();
}
