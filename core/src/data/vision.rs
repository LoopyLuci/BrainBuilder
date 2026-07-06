// Folder-of-images ingestion: turns the single most common shape of real
// personal training data — a folder whose subfolders are class names, each
// holding image files (`cats/`, `dogs/`, ...) — into the exact `RecordBatch`
// shape the existing training path already consumes: one row per image,
// `px0..px{F-1}` flattened normalized pixel columns, and a final `target`
// column holding the class index. No new tensor plumbing is needed beyond
// this loader: `ExecutionPlan::bind_data_ports` already stacks many feature
// columns into one `(batch, features)` matrix (what a `linear`/MLP classifier
// wants) and treats the last column as the target (see runtime::scheduler).
//
// This is deliberately the *flattened-features* path (a real, working image
// classifier via an MLP), not a `(batch, C, H, W)` conv path — the latter
// needs a shape-carrying column contract that doesn't exist yet and is called
// out honestly in `feature_layout`'s docs rather than faked here.
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use arrow::array::Float32Array;
use arrow::datatypes::{DataType as ArrowDataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The image file extensions we actually decode (matches the `image` crate
/// features enabled in Cargo.toml — enabling a format here without the codec
/// would just produce a decode error at load time, so the two are kept in
/// sync deliberately).
const SUPPORTED_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif"];

/// How each image is turned into a feature row. `size` is the square edge
/// length every image is resized to (so all rows are the same width, which a
/// dense classifier requires); `grayscale` collapses to 1 channel instead of
/// 3. The resulting per-row feature count is `size * size * channels`.
#[derive(Debug, Clone, Copy)]
pub struct ImageLayout {
    pub size: u32,
    pub grayscale: bool,
}

impl ImageLayout {
    pub fn channels(&self) -> usize {
        if self.grayscale { 1 } else { 3 }
    }

    /// Flattened feature count per image — the number of `px*` columns each
    /// row carries. Documented here (rather than assumed at call sites) because
    /// it's also exactly the `in_features` a downstream `linear` node must be
    /// configured with; the Intent layer reads this to size the model.
    pub fn feature_count(&self) -> usize {
        (self.size as usize) * (self.size as usize) * self.channels()
    }
}

impl Default for ImageLayout {
    fn default() -> Self {
        // 32x32 RGB: small enough to train quickly on CPU (the personal-first
        // default target), large enough to carry real visual signal.
        Self { size: 32, grayscale: false }
    }
}

/// The result of scanning + decoding an image folder: the class-name→index
/// mapping (so the GUI can show real labels, not bare integers, and so
/// inference can map a predicted index back to a name), the per-image feature
/// count, and the decoded batches ready for training.
#[derive(Debug)]
pub struct ImageDataset {
    /// Class names in label-index order: `class_names[0]` is the class whose
    /// target value is `0.0`, and so on. Sorted for determinism, so the same
    /// folder always yields the same label mapping across runs and machines.
    pub class_names: Vec<String>,
    pub feature_count: usize,
    pub batches: Vec<RecordBatch>,
}

/// One decoded, normalized image as a flat `f32` feature vector plus its class
/// index — the atomic unit the batching step groups into `RecordBatch`es.
struct Example {
    features: Vec<f32>,
    label: usize,
}

/// Scan `root` for immediate subdirectories (each a class), decode every
/// supported image inside into a normalized feature row, and pack the result
/// into `batch_size`-row `RecordBatch`es. Pixels are normalized to `[0, 1]`
/// (raw `u8 / 255.0`) — the standard, assumption-free starting normalization;
/// per-channel mean/std standardization is a separate, dataset-dependent step
/// left to `etl` rather than baked in here.
pub fn load_image_folder(
    root: &Path,
    layout: ImageLayout,
    batch_size: usize,
) -> Result<ImageDataset> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);

    if !root.is_dir() {
        return Err(cfg(format!("image folder `{}` is not a directory", root.display())));
    }

    // Discover classes: immediate subdirectories, sorted by name for a
    // deterministic, machine-independent label mapping.
    let mut class_dirs: Vec<(String, PathBuf)> = std::fs::read_dir(root)
        .map_err(|e| cfg(format!("cannot read image folder `{}`: {e}", root.display())))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.is_dir())
        .filter_map(|path| {
            path.file_name()
                .and_then(|n| n.to_str())
                .map(|name| (name.to_string(), path.clone()))
        })
        .collect();
    class_dirs.sort_by(|a, b| a.0.cmp(&b.0));

    if class_dirs.len() < 2 {
        return Err(cfg(format!(
            "image folder `{}` has {} class subfolder(s); need at least 2 (one directory per class, e.g. `cats/` and `dogs/`)",
            root.display(),
            class_dirs.len()
        )));
    }

    let class_names: Vec<String> = class_dirs.iter().map(|(name, _)| name.clone()).collect();
    let feature_count = layout.feature_count();

    // Decode every image in class order, then in filename order within a
    // class — deterministic, and it keeps a class's examples contiguous which
    // is fine because the trainer shuffles by shuffling the epoch's batch
    // order at a higher level (and single-batch personal datasets are common).
    let mut examples: Vec<Example> = Vec::new();
    for (label, (class_name, dir)) in class_dirs.iter().enumerate() {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir)
            .map_err(|e| cfg(format!("cannot read class folder `{}`: {e}", dir.display())))?
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| is_supported_image(path))
            .collect();
        files.sort();

        for path in files {
            let features = decode_to_features(&path, layout)?;
            debug_assert_eq!(features.len(), feature_count);
            examples.push(Example { features, label });
        }

        if !examples.iter().any(|e| e.label == label) {
            return Err(cfg(format!(
                "class `{class_name}` (`{}`) has no decodable images (supported: {})",
                dir.display(),
                SUPPORTED_EXTENSIONS.join(", ")
            )));
        }
    }

    let batch_size = batch_size.max(1);
    let batches = examples
        .chunks(batch_size)
        .map(|chunk| examples_to_record_batch(chunk, feature_count))
        .collect::<Result<Vec<_>>>()?;

    Ok(ImageDataset { class_names, feature_count, batches })
}

/// Count the decodable image files in each class subfolder (`(class, count)`,
/// sorted by class) without decoding anything — cheap enough to run at
/// propose/diagnose time to check class balance.
pub fn class_counts(root: &Path) -> Result<Vec<(String, usize)>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    if !root.is_dir() {
        return Err(cfg(format!("`{}` is not a directory", root.display())));
    }
    let mut counts: Vec<(String, usize)> = Vec::new();
    for entry in std::fs::read_dir(root).map_err(|e| cfg(e.to_string()))?.filter_map(|e| e.ok()) {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Some(name) = dir.file_name().and_then(|n| n.to_str()).map(String::from) else {
            continue;
        };
        let count = std::fs::read_dir(&dir)
            .map_err(|e| cfg(e.to_string()))?
            .filter_map(|e| e.ok())
            .filter(|e| is_supported_image(&e.path()))
            .count();
        counts.push((name, count));
    }
    counts.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(counts)
}

fn is_supported_image(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
            .unwrap_or(false)
}

/// Decode one image file, resize to the layout's square size, and flatten to a
/// normalized `f32` feature vector. Channel order is row-major
/// `[y][x][channel]` — the natural flattening of the resized buffer; the exact
/// order doesn't matter for a dense classifier as long as it's consistent
/// across every image (it is), which is why this is a valid MLP input.
fn decode_to_features(path: &Path, layout: ImageLayout) -> Result<Vec<f32>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let img = image::open(path)
        .map_err(|e| cfg(format!("failed to decode image `{}`: {e}", path.display())))?;

    // `Triangle` (bilinear) is a real, cheap, quality-adequate resize filter —
    // not nearest-neighbor (which would alias small images badly), not
    // Lanczos3 (slower, unnecessary at these small training sizes).
    let resized = img.resize_exact(layout.size, layout.size, image::imageops::FilterType::Triangle);

    let mut features = Vec::with_capacity(layout.feature_count());
    if layout.grayscale {
        let buf = resized.to_luma8();
        for pixel in buf.pixels() {
            features.push(pixel.0[0] as f32 / 255.0);
        }
    } else {
        let buf = resized.to_rgb8();
        for pixel in buf.pixels() {
            features.push(pixel.0[0] as f32 / 255.0);
            features.push(pixel.0[1] as f32 / 255.0);
            features.push(pixel.0[2] as f32 / 255.0);
        }
    }
    Ok(features)
}

fn examples_to_record_batch(chunk: &[Example], feature_count: usize) -> Result<RecordBatch> {
    // Column-major layout: one `Float32Array` per feature index, plus the
    // trailing `target` column — the exact `px0..px{F-1}, target` shape
    // `bind_data_ports`/`train_step` already consume.
    let mut columns: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk.len()); feature_count + 1];
    for example in chunk {
        for (i, value) in example.features.iter().enumerate() {
            columns[i].push(*value);
        }
        columns[feature_count].push(example.label as f32);
    }

    let fields: Vec<Field> = (0..feature_count)
        .map(|i| Field::new(format!("px{i}"), ArrowDataType::Float32, false))
        .chain(std::iter::once(Field::new("target", ArrowDataType::Float32, false)))
        .collect();
    let schema = Arc::new(Schema::new(fields));
    let arrays: Vec<Arc<dyn arrow::array::Array>> = columns
        .into_iter()
        .map(|c| Arc::new(Float32Array::from(c)) as Arc<dyn arrow::array::Array>)
        .collect();
    RecordBatch::try_new(schema, arrays).map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    /// Write a real solid-color PNG to disk — a genuine encoded image file the
    /// `image` crate decodes back through the exact same path a user's photo
    /// would, not an in-memory shortcut.
    fn write_png(path: &Path, w: u32, h: u32, color: [u8; 3]) {
        let mut img = RgbImage::new(w, h);
        for pixel in img.pixels_mut() {
            *pixel = Rgb(color);
        }
        img.save(path).expect("write test png");
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bb_vision_test_{name}_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn ingests_a_real_two_class_image_folder() {
        let root = temp_dir("two_class");
        let red = root.join("red");
        let blue = root.join("blue");
        std::fs::create_dir_all(&red).unwrap();
        std::fs::create_dir_all(&blue).unwrap();
        // 3 red images, 2 blue images, at odd source sizes to prove the resize
        // actually runs.
        write_png(&red.join("a.png"), 8, 8, [255, 0, 0]);
        write_png(&red.join("b.png"), 10, 6, [250, 5, 5]);
        write_png(&red.join("c.png"), 4, 12, [240, 10, 0]);
        write_png(&blue.join("a.png"), 8, 8, [0, 0, 255]);
        write_png(&blue.join("b.png"), 5, 5, [5, 5, 250]);

        let layout = ImageLayout { size: 4, grayscale: false };
        let dataset = load_image_folder(&root, layout, 16).unwrap();

        // Classes discovered and sorted: "blue" -> 0, "red" -> 1.
        assert_eq!(dataset.class_names, vec!["blue".to_string(), "red".to_string()]);
        assert_eq!(dataset.feature_count, 4 * 4 * 3);

        // 5 images total, one batch.
        assert_eq!(dataset.batches.len(), 1);
        let batch = &dataset.batches[0];
        assert_eq!(batch.num_rows(), 5);
        // px0..px47 + target
        assert_eq!(batch.num_columns(), 4 * 4 * 3 + 1);

        // The target column really holds the class indices (2 blue=0, 3 red=1).
        let target = batch
            .column(batch.num_columns() - 1)
            .as_any()
            .downcast_ref::<Float32Array>()
            .unwrap();
        let labels: Vec<f32> = (0..target.len()).map(|i| target.value(i)).collect();
        let blues = labels.iter().filter(|&&l| l == 0.0).count();
        let reds = labels.iter().filter(|&&l| l == 1.0).count();
        assert_eq!((blues, reds), (2, 3));

        // A red image's red channel (feature index 0, since channel order is
        // R,G,B per pixel) really is near-max after normalization.
        let first_red_row = labels.iter().position(|&l| l == 1.0).unwrap();
        let px0 = batch.column(0).as_any().downcast_ref::<Float32Array>().unwrap();
        assert!(px0.value(first_red_row) > 0.8, "red image's R channel should normalize near 1.0");

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn grayscale_layout_produces_single_channel_features() {
        let root = temp_dir("gray");
        for class in ["dark", "light"] {
            let dir = root.join(class);
            std::fs::create_dir_all(&dir).unwrap();
            let shade = if class == "dark" { 20 } else { 230 };
            write_png(&dir.join("x.png"), 6, 6, [shade, shade, shade]);
        }
        let layout = ImageLayout { size: 5, grayscale: true };
        let dataset = load_image_folder(&root, layout, 8).unwrap();
        assert_eq!(dataset.feature_count, 5 * 5); // 1 channel
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rejects_a_folder_with_fewer_than_two_classes() {
        let root = temp_dir("one_class");
        let only = root.join("only");
        std::fs::create_dir_all(&only).unwrap();
        write_png(&only.join("a.png"), 8, 8, [1, 2, 3]);
        let err = load_image_folder(&root, ImageLayout::default(), 8).unwrap_err();
        assert!(err.to_string().contains("at least 2"), "got: {err}");
        std::fs::remove_dir_all(&root).ok();
    }
}
