// Proves the data-time diagnostics run end to end against real data through
// the exact `intent::diagnose_data` path the GUI command calls — not just the
// pure rules in isolation. Builds a genuinely imbalanced image folder on disk
// (many of one class, a handful of another) and asserts the imbalance + tiny-
// class warnings actually fire. No torch needed.
use brainbuilder_core::diagnostics::Severity;
use brainbuilder_core::intent::{diagnose_data, DataSpec, IntentRequest, TaskKind};
use image::{Rgb, RgbImage};
use std::path::Path;

fn write_png(path: &Path, color: [u8; 3]) {
    let mut img = RgbImage::new(4, 4);
    for p in img.pixels_mut() {
        *p = Rgb(color);
    }
    img.save(path).unwrap();
}

#[test]
fn diagnose_data_flags_a_real_imbalanced_image_folder() {
    let root = std::env::temp_dir().join(format!("bb_diag_int_{}", uuid::Uuid::new_v4()));
    let common = root.join("common");
    let rare = root.join("rare");
    std::fs::create_dir_all(&common).unwrap();
    std::fs::create_dir_all(&rare).unwrap();
    // 40 common vs 2 rare: severely imbalanced *and* the rare class is tiny.
    for i in 0..40 {
        write_png(&common.join(format!("{i}.png")), [10, 10, 10]);
    }
    for i in 0..2 {
        write_png(&rare.join(format!("{i}.png")), [200, 200, 200]);
    }

    let request = IntentRequest {
        task: TaskKind::Classification,
        data: DataSpec {
            source_type: "image_folder".to_string(),
            path: root.to_string_lossy().to_string(),
            image_size: Some(8),
            grayscale: None,
            augment: None,
            text_column: None,
            label_column: None,
            vocab_size: None,
        },
    };

    let rt = tokio::runtime::Runtime::new().unwrap();
    let diags = rt.block_on(diagnose_data(&request)).expect("diagnose_data should succeed");

    assert!(
        diags.iter().any(|d| d.title.contains("imbalanced") && d.severity == Severity::Warning),
        "expected an imbalance warning, got: {:?}",
        diags.iter().map(|d| &d.title).collect::<Vec<_>>()
    );
    assert!(
        diags.iter().any(|d| d.title.contains("few examples")),
        "expected a tiny-class warning, got: {:?}",
        diags.iter().map(|d| &d.title).collect::<Vec<_>>()
    );

    std::fs::remove_dir_all(&root).ok();
}
