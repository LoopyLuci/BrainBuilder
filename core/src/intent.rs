//! The task-first "Intent" layer — BrainBuilder's on-ramp for someone who
//! thinks in *outcomes and data*, not layers and graphs.
//!
//! A zero-knowledge user says "I have a folder of photos labeled cat/dog, tell
//! them apart" — not "attention + layernorm + linear". This module turns that
//! kind of intent (a task kind + a pointer at real data) into a **validated,
//! trainable** BBIR graph sized to the data, plus a plain-English rationale.
//! The node graph everything else in this app operates on becomes an optional
//! "open the hood" view rather than the price of entry.
//!
//! What it does, concretely:
//! 1. **Inspects the real data** to learn its shape — how many input features,
//!    how many classes — by actually reading the folder / spreadsheet (reusing
//!    the exact ingestion code the trainer uses, so the proposal can't drift
//!    from what training will really see).
//! 2. **Assembles a known-good architecture** sized to that shape (a small MLP
//!    for classification, a linear model for regression) out of the same real
//!    components the palette offers.
//! 3. **Validates it** through the exact `validate_graph` the canvas and
//!    `execute_graph` use — so a proposal that couldn't train is caught here,
//!    not handed to the user as a broken graph.
//!
//! This is deliberately a small, honest set of architectures — the floor that
//! makes the existing engine reachable, not a pretense of covering every model
//! ever. Transfer-learning proposals (adapting a pretrained model) are built
//! separately in `intent::transfer` and slot in behind this same interface.
use crate::bbir::{BBIREdge, BBIRGraph, BBIRNode, DataSourceConfig, PortInfo, TrainingConfig};
use crate::component::registry::ComponentRegistry;
use crate::component::validation::validate_graph;
use crate::interop::protocol::BrainBuilderError;
use crate::Result;

/// The kind of outcome the user wants — the vocabulary of *goals*, not
/// architectures. Kept intentionally small and concrete: each maps to a real,
/// tested loss + output shape in this engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskKind {
    /// Assign each example to one of N discrete classes (cross-entropy).
    Classification,
    /// Predict a single continuous number per example (MSE).
    Regression,
}

/// Where the data lives and how to read it — mirrors the fields the trainer's
/// `DataSourceConfig` needs, but framed as "what the user pointed at".
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DataSpec {
    /// One of `image_folder`, `text_column`, `file` (numeric csv/parquet).
    pub source_type: String,
    pub path: String,
    #[serde(default)]
    pub image_size: Option<usize>,
    #[serde(default)]
    pub grayscale: Option<bool>,
    #[serde(default)]
    pub text_column: Option<String>,
    #[serde(default)]
    pub label_column: Option<String>,
    #[serde(default)]
    pub vocab_size: Option<usize>,
}

/// A full intent: the goal plus the data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct IntentRequest {
    pub task: TaskKind,
    pub data: DataSpec,
}

/// The result the user gets back: a ready-to-train graph, the class names it
/// discovered (empty for regression), the input/output sizes it chose, and a
/// plain-English explanation of what it built and why — so even the "open the
/// hood" moment is legible.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ProposedModel {
    pub graph: BBIRGraph,
    pub class_names: Vec<String>,
    pub feature_count: usize,
    pub num_outputs: usize,
    pub rationale: String,
}

/// What inspecting the real data told us about its shape. Public so the GUI's
/// Tauri command can inspect (async, no registry lock) and then finalize
/// (sync, holding the registry lock only briefly) in two steps — the registry
/// guard is a non-`Send` `std::sync::RwLock`, so it must never be held across
/// the `.await` that reads the data.
#[derive(Debug, Clone)]
pub struct DataShape {
    pub feature_count: usize,
    /// Discovered class names, sorted. Empty for regression.
    pub class_names: Vec<String>,
    /// A human-facing note about how the data was read (row/class counts).
    pub summary: String,
}

/// Turn an intent into a validated, trainable model proposal. Async because
/// inspecting tabular/text data reads it through DataFusion (the same reader
/// training uses). This is the convenience entry point (used by tests and any
/// caller that can hold the registry across the await); the GUI uses
/// `inspect_data` + `finalize_proposal` instead to respect its lock's
/// non-`Send`ness.
pub async fn propose_model(request: &IntentRequest, registry: &ComponentRegistry) -> Result<ProposedModel> {
    let shape = inspect_data(request).await?;
    finalize_proposal(request, shape, registry)
}

/// The sync half of `propose_model`: size and assemble the architecture from
/// an already-inspected `DataShape`, then validate it against the registry.
/// Holds no locks and does no I/O, so a caller can invoke it while holding a
/// non-`Send` registry guard without crossing an `.await`.
pub fn finalize_proposal(
    request: &IntentRequest,
    shape: DataShape,
    registry: &ComponentRegistry,
) -> Result<ProposedModel> {
    let num_outputs = match request.task {
        TaskKind::Classification => {
            if shape.class_names.len() < 2 {
                return Err(BrainBuilderError::ConfigError(format!(
                    "classification needs at least 2 classes, but the data has {} ({})",
                    shape.class_names.len(),
                    shape.summary
                )));
            }
            shape.class_names.len()
        }
        // A single continuous output.
        TaskKind::Regression => 1,
    };

    let (graph, rationale) = build_graph(request, &shape, num_outputs)?;

    // The proposal must pass the exact validation real training runs — a
    // model we can't stand behind is worse than none.
    validate_graph(&graph, registry)?;

    Ok(ProposedModel {
        graph,
        class_names: shape.class_names,
        feature_count: shape.feature_count,
        num_outputs,
        rationale,
    })
}

/// Run the data-time diagnostics for an intent: read the real data and apply
/// the pure rules in `crate::diagnostics` (class balance for classification,
/// numeric feature/target leakage for tabular). Returns a plain-English list
/// the GUI shows before the user commits to a training run — the "will this
/// even work?" check. Keeps all the I/O here so `diagnostics` stays pure.
pub async fn diagnose_data(request: &IntentRequest) -> Result<Vec<crate::diagnostics::Diagnostic>> {
    use crate::diagnostics::{analyze_class_balance, analyze_feature_target_leakage};
    let data = &request.data;
    let mut diags = Vec::new();

    // Class balance (classification only): count examples per class.
    if request.task == TaskKind::Classification {
        let counts: Vec<(String, usize)> = match data.source_type.as_str() {
            "image_folder" => crate::data::vision::class_counts(std::path::Path::new(&data.path))?,
            "text_column" => {
                let rows = crate::data::source::read_text_label_rows(&data.to_source_config(0)).await?;
                let mut map: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
                for r in &rows {
                    *map.entry(r.label.clone()).or_insert(0) += 1;
                }
                map.into_iter().collect()
            }
            "file" => {
                let columns = crate::data::source::column_names(&data.path).await?;
                let label_col = data.label_column.clone().or_else(|| columns.last().cloned());
                match label_col {
                    Some(col) => crate::data::source::label_counts(&data.path, &col).await?,
                    None => Vec::new(),
                }
            }
            _ => Vec::new(),
        };
        diags.extend(analyze_class_balance(&counts));
    }

    // Feature/target leakage (numeric tabular files only).
    if data.source_type == "file" {
        let numeric = crate::data::source::numeric_columns(&data.path).await?;
        let columns = crate::data::source::column_names(&data.path).await?;
        let label_col = data.label_column.clone().or_else(|| columns.last().cloned());
        if let Some(target_name) = label_col {
            if let Some((_, target)) = numeric.iter().find(|(n, _)| n == &target_name) {
                for (name, feature) in &numeric {
                    if name == &target_name {
                        continue;
                    }
                    if let Some(d) = analyze_feature_target_leakage(name, feature, target) {
                        diags.push(d);
                    }
                }
            }
        }
    }

    Ok(diags)
}

/// A transfer-learning intent: adapt a real pretrained model to the user's own
/// data. Points at a `.safetensors` file and one of its 2-D weight tensors to
/// use as a frozen backbone, plus the usual task + data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TransferRequest {
    pub task: TaskKind,
    pub data: DataSpec,
    pub pretrained_file: String,
    pub pretrained_tensor: String,
}

/// Propose a transfer-learning model: a **frozen** pretrained backbone (a
/// `lora_linear` node whose base `weight` is seeded from the chosen tensor and
/// never updated) followed by a fresh, fully-trainable head sized to the
/// user's classes. This is "build any model by adapting a foundation model
/// instead of training from scratch" — the head learns on top of fixed
/// pretrained features, so it trains fast on little data.
///
/// Honest boundary: the backbone here is a single pretrained weight *matrix*
/// used as a linear feature extractor. Running a whole pretrained network
/// (a CNN/transformer with all its layers and nonlinearities) as the frozen
/// extractor is the documented next step that builds on this same seam
/// (`scheduler::load_preset_weights`) — it needs a component that reconstructs
/// the pretrained architecture, not just one of its tensors.
pub async fn propose_transfer_model(
    request: &TransferRequest,
    registry: &ComponentRegistry,
) -> Result<ProposedModel> {
    let shape = inspect_transfer(request).await?;
    finalize_transfer(request, shape, registry)
}

/// The backbone/data shape a transfer proposal is built from — separated so
/// the GUI can inspect (async, no registry lock) then finalize (sync, brief
/// lock), the same non-`Send` dance `propose_model` uses.
#[derive(Debug, Clone)]
pub struct TransferShape {
    backbone_in: i64,
    backbone_out: i64,
    num_outputs: usize,
    data: DataShape,
}

/// Async half of `propose_transfer_model`: read the pretrained file's manifest
/// and inspect the data, validating that the backbone's input dimension
/// matches the data's feature count. No registry needed.
pub async fn inspect_transfer(request: &TransferRequest) -> Result<TransferShape> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);

    // Backbone dimensions come from the real file's manifest (no weight data
    // loaded — just the header).
    let manifest = crate::models::safetensors_loader::list_safetensors_manifest(std::path::Path::new(
        &request.pretrained_file,
    ))?;
    let (_, shape, _) = manifest
        .iter()
        .find(|(name, _, _)| name == &request.pretrained_tensor)
        .ok_or_else(|| {
            cfg(format!(
                "tensor `{}` not found in `{}`. Available: {}",
                request.pretrained_tensor,
                request.pretrained_file,
                manifest.iter().map(|(n, _, _)| n.as_str()).collect::<Vec<_>>().join(", ")
            ))
        })?;
    if shape.len() != 2 {
        return Err(cfg(format!(
            "pretrained tensor `{}` has shape {:?}; the backbone must be a 2-D weight matrix [out, in]",
            request.pretrained_tensor, shape
        )));
    }
    let (backbone_out, backbone_in) = (shape[0], shape[1]);

    // The user's data must match what the backbone expects as input.
    let inspect_request = IntentRequest { task: request.task, data: request.data.clone() };
    let data_shape = inspect_data(&inspect_request).await?;
    if data_shape.feature_count as i64 != backbone_in {
        return Err(cfg(format!(
            "your data has {} input feature(s) per example, but the pretrained backbone `{}` expects {}. \
They must match — pick a backbone tensor whose input dimension equals your data's feature count.",
            data_shape.feature_count, request.pretrained_tensor, backbone_in
        )));
    }

    let num_outputs = match request.task {
        TaskKind::Classification => {
            if data_shape.class_names.len() < 2 {
                return Err(cfg(format!(
                    "classification needs at least 2 classes, but the data has {}",
                    data_shape.class_names.len()
                )));
            }
            data_shape.class_names.len()
        }
        TaskKind::Regression => 1,
    };

    Ok(TransferShape { backbone_in, backbone_out, num_outputs, data: data_shape })
}

/// Sync half of `propose_transfer_model`: assemble the frozen-backbone +
/// fresh-head graph and validate it against the registry. No I/O, no locks
/// held across an await.
pub fn finalize_transfer(
    request: &TransferRequest,
    shape: TransferShape,
    registry: &ComponentRegistry,
) -> Result<ProposedModel> {
    let TransferShape { backbone_in, backbone_out, num_outputs, data } = shape;
    let graph = build_transfer_graph(request, backbone_in, backbone_out, num_outputs, &data);
    validate_graph(&graph, registry)?;

    let (loss_word, class_note) = match request.task {
        TaskKind::Classification => (
            "classify",
            format!(" into {} classes ({})", num_outputs, preview_list(&data.class_names)),
        ),
        TaskKind::Regression => ("predict a number for", String::new()),
    };
    let rationale = format!(
        "Adapting the pretrained backbone `{}` ({backbone_in}→{backbone_out}) to {loss_word} your data{class_note}. \
The backbone is frozen (its weights stay fixed) and a fresh trainable head ({backbone_out}→{num_outputs}) learns on top of it — \
so it trains quickly even on little data. Detected from your data: {}.",
        request.pretrained_tensor, data.summary
    );

    Ok(ProposedModel {
        graph,
        class_names: data.class_names,
        feature_count: data.feature_count,
        num_outputs,
        rationale,
    })
}

/// Build the frozen-backbone → relu → fresh-head graph for a transfer request.
fn build_transfer_graph(
    request: &TransferRequest,
    backbone_in: i64,
    backbone_out: i64,
    num_outputs: usize,
    shape: &DataShape,
) -> BBIRGraph {
    let backbone = BBIRNode {
        id: "backbone".into(),
        component: "lora_linear".into(),
        label: Some("Pretrained (frozen)".into()),
        hyperparams: serde_json::json!({
            "in_features": backbone_in,
            "out_features": backbone_out,
            "rank": 4,
            "alpha": 8.0,
            "pretrained": {
                "file": request.pretrained_file,
                "tensor": request.pretrained_tensor,
                "port": "weight"
            }
        }),
        ports: PortInfo {
            input_ports: vec!["input".into(), "weight".into(), "lora_a".into(), "lora_b".into()],
            output_ports: vec!["output".into()],
        },
        position: None,
    };
    let act = relu_node("act");
    let head = linear_node("head", backbone_out, num_outputs as i64);

    let loss = match request.task {
        TaskKind::Classification => "cross_entropy",
        TaskKind::Regression => "mse",
    };

    BBIRGraph {
        schema_version: crate::bbir::CURRENT_BBIR_SCHEMA_VERSION,
        graph_id: uuid::Uuid::new_v4().to_string(),
        name: format!("{}-transfer", proposed_name(&IntentRequest { task: request.task, data: request.data.clone() })),
        nodes: vec![backbone, act, head],
        edges: vec![
            BBIREdge { from_node: "backbone".into(), from_port: "output".into(), to_node: "act".into(), to_port: "x".into() },
            BBIREdge { from_node: "act".into(), from_port: "y".into(), to_node: "head".into(), to_port: "input".into() },
        ],
        training: Some(TrainingConfig {
            loss: loss.to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            hyperparams: serde_json::json!({"lr": 0.001, "epochs": 40}),
            data_source: request.data.to_source_config(default_batch_size(shape)),
            reproducibility: None,
        }),
    }
}

/// Inspect the real data to learn its feature count and (for classification)
/// its class list, reusing the trainer's own ingestion so the numbers can't
/// diverge from what training will see. Public so the GUI can run this async
/// step before acquiring its non-`Send` registry lock.
pub async fn inspect_data(request: &IntentRequest) -> Result<DataShape> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let data = &request.data;
    match data.source_type.as_str() {
        "image_folder" => {
            let layout = crate::data::vision::ImageLayout {
                size: data.image_size.unwrap_or(32) as u32,
                grayscale: data.grayscale.unwrap_or(false),
            };
            // Scan classes cheaply (no decode) from immediate subdirectories.
            let root = std::path::Path::new(&data.path);
            if !root.is_dir() {
                return Err(cfg(format!("`{}` is not a folder of image class subfolders", data.path)));
            }
            let mut class_names: Vec<String> = std::fs::read_dir(root)
                .map_err(|e| cfg(format!("cannot read `{}`: {e}", data.path)))?
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_dir())
                .filter_map(|p| p.file_name().and_then(|n| n.to_str()).map(String::from))
                .collect();
            class_names.sort();
            let feature_count = layout.feature_count();
            let summary = format!(
                "{} class folder(s), each image resized to {}x{} {}",
                class_names.len(),
                layout.size,
                layout.size,
                if layout.grayscale { "grayscale" } else { "RGB" }
            );
            Ok(DataShape { feature_count, class_names, summary })
        }
        "text_column" => {
            let rows = crate::data::source::read_text_label_rows(&data.to_source_config(0)).await?;
            let vocab_size = data.vocab_size.unwrap_or(2_000);
            let dataset = crate::data::tabular_text::build_bag_of_words(&rows, vocab_size)?;
            let summary = format!(
                "{} row(s) of text, {}-word vocabulary, {} class(es)",
                rows.len(),
                dataset.feature_count,
                dataset.class_names.len()
            );
            Ok(DataShape {
                feature_count: dataset.feature_count,
                class_names: dataset.class_names,
                summary,
            })
        }
        "file" => {
            let columns = crate::data::source::column_names(&data.path).await?;
            if columns.len() < 2 {
                return Err(cfg(format!(
                    "tabular file needs at least 2 columns (features + a target), found {}",
                    columns.len()
                )));
            }
            let label_col = data.label_column.clone().unwrap_or_else(|| columns.last().cloned().unwrap());
            // Feature columns = everything except the label column.
            let feature_count = columns.iter().filter(|c| *c != &label_col).count();
            let class_names = match request.task {
                TaskKind::Classification => crate::data::source::distinct_string_values(&data.path, &label_col).await?,
                TaskKind::Regression => Vec::new(),
            };
            let summary = format!(
                "{} feature column(s), target column `{}`{}",
                feature_count,
                label_col,
                match request.task {
                    TaskKind::Classification => format!(", {} class(es)", class_names.len()),
                    TaskKind::Regression => String::new(),
                }
            );
            Ok(DataShape { feature_count, class_names, summary })
        }
        other => Err(cfg(format!("Intent layer doesn't yet know how to inspect data source `{other}`"))),
    }
}

/// Assemble a known-good architecture sized to the data. Classification and
/// regression both use a small 2-layer MLP (linear → relu → linear): enough
/// capacity to learn real non-linear structure, small enough to train quickly
/// on CPU (the personal-first default). The hidden width is a modest function
/// of the input size, clamped to a sane personal-scale range.
fn build_graph(request: &IntentRequest, shape: &DataShape, num_outputs: usize) -> Result<(BBIRGraph, String)> {
    let feature_count = shape.feature_count as i64;
    let hidden = hidden_width(shape.feature_count);
    let (loss, task_word) = match request.task {
        TaskKind::Classification => ("cross_entropy", "classify"),
        TaskKind::Regression => ("mse", "predict a number from"),
    };

    let nodes = vec![
        linear_node("fc1", feature_count, hidden as i64),
        relu_node("act"),
        linear_node("fc2", hidden as i64, num_outputs as i64),
    ];
    let edges = vec![
        BBIREdge { from_node: "fc1".into(), from_port: "output".into(), to_node: "act".into(), to_port: "x".into() },
        BBIREdge { from_node: "act".into(), from_port: "y".into(), to_node: "fc2".into(), to_port: "input".into() },
    ];

    let data_source = request.data.to_source_config(default_batch_size(shape));

    let graph = BBIRGraph {
        schema_version: crate::bbir::CURRENT_BBIR_SCHEMA_VERSION,
        graph_id: uuid::Uuid::new_v4().to_string(),
        name: proposed_name(request),
        nodes,
        edges,
        training: Some(TrainingConfig {
            loss: loss.to_string(),
            optimizer: "adam".to_string(),
            trainer_type: "standard".to_string(),
            // Adam @ 1e-3 for a modest number of epochs: a robust,
            // widely-applicable default that "just works" on personal-scale
            // data without the user tuning anything.
            hyperparams: serde_json::json!({"lr": 0.001, "epochs": 40}),
            data_source,
            reproducibility: None,
        }),
    };

    let class_note = if shape.class_names.is_empty() {
        String::new()
    } else {
        format!(" into {} classes ({})", shape.class_names.len(), preview_list(&shape.class_names))
    };
    let rationale = format!(
        "Built a small 2-layer neural network (a {feature_count}→{hidden}→{num_outputs} multi-layer perceptron) to {task_word} your data{class_note}. \
Detected from your data: {}. It trains with the Adam optimizer at a learning rate of 0.001 for 40 passes — sensible defaults you can adjust. \
Open the canvas any time to see and edit the underlying graph.",
        shape.summary
    );

    Ok((graph, rationale))
}

/// A hidden layer width that scales gently with input size but stays in a
/// personal-scale, CPU-friendly range.
fn hidden_width(feature_count: usize) -> usize {
    let scaled = (feature_count as f64).sqrt() as usize * 4;
    scaled.clamp(16, 256)
}

/// Small datasets want to see all their data each step; larger ones want
/// mini-batches. A simple, safe heuristic.
fn default_batch_size(shape: &DataShape) -> usize {
    // We don't always know the row count cheaply (image folders aren't fully
    // decoded during inspection), so pick a conservative, universally-safe
    // mini-batch size.
    let _ = shape;
    32
}

fn proposed_name(request: &IntentRequest) -> String {
    let kind = match request.task {
        TaskKind::Classification => "classifier",
        TaskKind::Regression => "regressor",
    };
    let base = std::path::Path::new(&request.data.path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    format!("{base}-{kind}")
}

fn linear_node(id: &str, in_features: i64, out_features: i64) -> BBIRNode {
    BBIRNode {
        id: id.to_string(),
        component: "linear".to_string(),
        label: None,
        hyperparams: serde_json::json!({"in_features": in_features, "out_features": out_features}),
        ports: PortInfo {
            input_ports: vec!["input".to_string(), "weight".to_string()],
            output_ports: vec!["output".to_string()],
        },
        position: None,
    }
}

fn relu_node(id: &str) -> BBIRNode {
    BBIRNode {
        id: id.to_string(),
        component: "relu".to_string(),
        label: None,
        hyperparams: serde_json::json!({}),
        ports: PortInfo { input_ports: vec!["x".to_string()], output_ports: vec!["y".to_string()] },
        position: None,
    }
}

fn preview_list(items: &[String]) -> String {
    const MAX: usize = 6;
    if items.len() <= MAX {
        items.join(", ")
    } else {
        format!("{}, …", items[..MAX].join(", "))
    }
}

impl DataSpec {
    /// Project this user-facing spec onto the trainer's `DataSourceConfig`.
    fn to_source_config(&self, batch_size: usize) -> DataSourceConfig {
        DataSourceConfig {
            source_type: self.source_type.clone(),
            path_or_uri: self.path.clone(),
            batch_size,
            preprocessing: Vec::new(),
            sequence_length: None,
            vocab_size: self.vocab_size,
            image_size: self.image_size,
            grayscale: self.grayscale,
            text_column: self.text_column.clone(),
            label_column: self.label_column.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{Rgb, RgbImage};

    fn real_registry() -> ComponentRegistry {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../components");
        let mut registry = ComponentRegistry::new();
        registry.load_from_dir(&dir).expect("real components must load");
        registry
    }

    fn write_png(path: &std::path::Path, color: [u8; 3]) {
        let mut img = RgbImage::new(6, 6);
        for p in img.pixels_mut() {
            *p = Rgb(color);
        }
        img.save(path).unwrap();
    }

    #[test]
    fn proposes_a_valid_image_classifier_sized_to_the_folder() {
        let root = std::env::temp_dir().join(format!("bb_intent_img_{}", uuid::Uuid::new_v4()));
        for class in ["cat", "dog", "bird"] {
            let d = root.join(class);
            std::fs::create_dir_all(&d).unwrap();
            write_png(&d.join("a.png"), [10, 20, 30]);
        }
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

        let registry = real_registry();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proposal = rt.block_on(propose_model(&request, &registry)).expect("should propose a valid model");

        // 3 sorted classes, input sized to 16*16*3.
        assert_eq!(proposal.class_names, vec!["bird".to_string(), "cat".to_string(), "dog".to_string()]);
        assert_eq!(proposal.feature_count, 16 * 16 * 3);
        assert_eq!(proposal.num_outputs, 3);

        // The proposed graph really is the MLP we described, and its input
        // linear layer is sized to the real feature count.
        let fc1 = proposal.graph.nodes.iter().find(|n| n.id == "fc1").unwrap();
        assert_eq!(fc1.hyperparams["in_features"], 16 * 16 * 3);
        let fc2 = proposal.graph.nodes.iter().find(|n| n.id == "fc2").unwrap();
        assert_eq!(fc2.hyperparams["out_features"], 3);

        // The training config points back at the real folder with the right
        // ingestion settings.
        let training = proposal.graph.training.as_ref().unwrap();
        assert_eq!(training.loss, "cross_entropy");
        assert_eq!(training.data_source.source_type, "image_folder");
        assert_eq!(training.data_source.image_size, Some(16));

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn proposes_a_valid_regressor_for_a_numeric_csv() {
        let dir = std::env::temp_dir().join(format!("bb_intent_tab_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let csv = dir.join("houses.csv");
        std::fs::write(&csv, "sqft,beds,price\n1000,2,200000\n1500,3,300000\n2000,4,400000\n").unwrap();

        let request = IntentRequest {
            task: TaskKind::Regression,
            data: DataSpec {
                source_type: "file".to_string(),
                path: csv.to_string_lossy().to_string(),
                image_size: None,
                grayscale: None,
                text_column: None,
                label_column: Some("price".to_string()),
                vocab_size: None,
            },
        };
        let registry = real_registry();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proposal = rt.block_on(propose_model(&request, &registry)).expect("should propose a regressor");

        assert!(proposal.class_names.is_empty(), "regression has no classes");
        assert_eq!(proposal.feature_count, 2); // sqft, beds (price is the target)
        assert_eq!(proposal.num_outputs, 1);
        assert_eq!(proposal.graph.training.as_ref().unwrap().loss, "mse");

        std::fs::remove_dir_all(&dir).ok();
    }

    fn write_backbone_safetensors(path: &std::path::Path, out: usize, in_: usize) {
        use safetensors::tensor::TensorView;
        use safetensors::Dtype;
        let values: Vec<f32> = (0..out * in_).map(|i| ((i as f32) * 0.31).sin() * 0.5).collect();
        let bytes: Vec<u8> = values.iter().flat_map(|f| f.to_le_bytes()).collect();
        let view = TensorView::new(Dtype::F32, vec![out, in_], &bytes).unwrap();
        let mut map = std::collections::HashMap::new();
        map.insert("backbone.weight".to_string(), view);
        safetensors::serialize_to_file(&map, &None, path).unwrap();
    }

    #[test]
    fn proposes_a_valid_frozen_backbone_transfer_model_sized_to_the_data() {
        // Grayscale 8x8 images => 64 input features. Backbone must expect 64 in.
        let root = std::env::temp_dir().join(format!("bb_transfer_intent_{}", uuid::Uuid::new_v4()));
        for class in ["a", "b"] {
            let d = root.join(class);
            std::fs::create_dir_all(&d).unwrap();
            write_png(&d.join("x.png"), if class == "a" { [10, 10, 10] } else { [240, 240, 240] });
        }
        let st = root.join("backbone.safetensors");
        write_backbone_safetensors(&st, 16, 64); // [out=16, in=64]

        let request = TransferRequest {
            task: TaskKind::Classification,
            data: DataSpec {
                source_type: "image_folder".to_string(),
                path: root.to_string_lossy().to_string(),
                image_size: Some(8),
                grayscale: Some(true),
                text_column: None,
                label_column: None,
                vocab_size: None,
            },
            pretrained_file: st.to_string_lossy().to_string(),
            pretrained_tensor: "backbone.weight".to_string(),
        };

        let registry = real_registry();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let proposal = rt.block_on(propose_transfer_model(&request, &registry)).expect("should propose transfer model");

        // Frozen backbone sized to the pretrained tensor; head sized to classes.
        let backbone = proposal.graph.nodes.iter().find(|n| n.id == "backbone").unwrap();
        assert_eq!(backbone.component, "lora_linear");
        assert_eq!(backbone.hyperparams["in_features"], 64);
        assert_eq!(backbone.hyperparams["out_features"], 16);
        assert!(backbone.hyperparams["pretrained"]["tensor"] == "backbone.weight");
        let head = proposal.graph.nodes.iter().find(|n| n.id == "head").unwrap();
        assert_eq!(head.hyperparams["in_features"], 16);
        assert_eq!(head.hyperparams["out_features"], 2);

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn rejects_a_backbone_whose_input_doesnt_match_the_data() {
        let root = std::env::temp_dir().join(format!("bb_transfer_bad_{}", uuid::Uuid::new_v4()));
        for class in ["a", "b"] {
            let d = root.join(class);
            std::fs::create_dir_all(&d).unwrap();
            write_png(&d.join("x.png"), [1, 2, 3]);
        }
        let st = root.join("backbone.safetensors");
        write_backbone_safetensors(&st, 16, 100); // expects 100 inputs, data has 64

        let request = TransferRequest {
            task: TaskKind::Classification,
            data: DataSpec {
                source_type: "image_folder".to_string(),
                path: root.to_string_lossy().to_string(),
                image_size: Some(8),
                grayscale: Some(true),
                text_column: None,
                label_column: None,
                vocab_size: None,
            },
            pretrained_file: st.to_string_lossy().to_string(),
            pretrained_tensor: "backbone.weight".to_string(),
        };
        let registry = real_registry();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(propose_transfer_model(&request, &registry)).unwrap_err().to_string();
        assert!(err.contains("They must match"), "expected a dimension-mismatch error, got: {err}");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_classification_with_only_one_class() {
        let root = std::env::temp_dir().join(format!("bb_intent_one_{}", uuid::Uuid::new_v4()));
        let d = root.join("only");
        std::fs::create_dir_all(&d).unwrap();
        write_png(&d.join("a.png"), [1, 2, 3]);
        let request = IntentRequest {
            task: TaskKind::Classification,
            data: DataSpec {
                source_type: "image_folder".to_string(),
                path: root.to_string_lossy().to_string(),
                image_size: Some(8),
                grayscale: None,
                text_column: None,
                label_column: None,
                vocab_size: None,
            },
        };
        let registry = real_registry();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let err = rt.block_on(propose_model(&request, &registry)).unwrap_err();
        assert!(err.to_string().contains("at least 2 classes"), "got: {err}");
        std::fs::remove_dir_all(&root).ok();
    }
}
