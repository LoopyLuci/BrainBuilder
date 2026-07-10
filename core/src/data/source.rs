use datafusion::prelude::*;
use arrow::record_batch::RecordBatch;
use arrow::util::display::{ArrayFormatter, FormatOptions};
use crate::Result;
use crate::interop::protocol::BrainBuilderError;

pub trait DataIterator: Send + Sync {
    fn next(&mut self) -> Result<Option<RecordBatch>>;
    /// Rewinds to the first batch. Real bug this exists to fix: without it,
    /// `StandardTrainer::fit`'s `for epoch in 0..epochs { while let Some(batch)
    /// = data.next()? {...} }` only ever consumed batches once — every epoch
    /// after the first silently ran zero real training steps (`next()` had
    /// already been exhausted), so the `epochs` hyperparameter did nothing
    /// beyond epoch 0 for any dataset small enough to fit in one DataFusion
    /// batch chunk (the common case for a personal-scale CSV — one batch
    /// means exactly one real step total, no matter how many `epochs` are
    /// configured). Caught by `branching_graph.rs`: a 30-row/batch_size-30
    /// dataset logged exactly one metrics point across 20 configured epochs,
    /// with `first_loss == last_loss` because there was only ever one real
    /// step to measure.
    fn reset(&mut self) -> Result<()>;
}

/// Read a csv/parquet file into a DataFusion `DataFrame`. Shared by
/// `load_dataset` (training) and `preview_dataset` (GUI table preview).
async fn read_file(ctx: &SessionContext, path: &str) -> Result<DataFrame> {
    if path.ends_with(".parquet") {
        ctx.read_parquet(path, ParquetReadOptions::default())
            .await
            .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
    } else {
        ctx.read_csv(path, CsvReadOptions::default())
            .await
            .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
    }
}

/// Read the free-text and label columns of a tabular file into `(text, label)`
/// rows, stringifying every cell through the same Arrow `ArrayFormatter` the
/// preview uses (so any inferred column type works). Shared by the
/// `text_column` training loader and the Intent layer's data inspection, so
/// the two can never disagree about how a spreadsheet is read.
pub async fn read_text_label_rows(
    config: &crate::bbir::DataSourceConfig,
) -> Result<Vec<crate::data::tabular_text::TextRow>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let text_col = config
        .text_column
        .as_ref()
        .ok_or_else(|| cfg("text_column source requires `text_column` to name the free-text column".into()))?;

    let ctx = SessionContext::new();
    let df = read_file(&ctx, &config.path_or_uri).await?;
    let columns: Vec<String> = df.schema().fields().iter().map(|f| f.name().clone()).collect();

    // Default label to the last column (the trainer's usual convention) when
    // not explicitly named.
    let label_col = config
        .label_column
        .clone()
        .or_else(|| columns.last().cloned())
        .ok_or_else(|| cfg("dataset has no columns".into()))?;

    let text_idx = columns
        .iter()
        .position(|c| c == text_col)
        .ok_or_else(|| cfg(format!("text column `{text_col}` not found (columns: {})", columns.join(", "))))?;
    let label_idx = columns
        .iter()
        .position(|c| c == &label_col)
        .ok_or_else(|| cfg(format!("label column `{label_col}` not found (columns: {})", columns.join(", "))))?;

    let batches = df.collect().await.map_err(|e| cfg(e.to_string()))?;
    let opts = FormatOptions::default();
    let mut rows: Vec<crate::data::tabular_text::TextRow> = Vec::new();
    for batch in &batches {
        let text_fmt = ArrayFormatter::try_new(batch.column(text_idx).as_ref(), &opts)
            .map_err(|e| cfg(e.to_string()))?;
        let label_fmt = ArrayFormatter::try_new(batch.column(label_idx).as_ref(), &opts)
            .map_err(|e| cfg(e.to_string()))?;
        for r in 0..batch.num_rows() {
            rows.push(crate::data::tabular_text::TextRow {
                text: text_fmt.value(r).to_string(),
                label: label_fmt.value(r).to_string(),
            });
        }
    }
    Ok(rows)
}

/// The column names of a tabular file, in order — used by the Intent layer to
/// size a tabular model (feature count = columns minus the label) and to let
/// the GUI offer real column choices.
pub async fn column_names(path: &str) -> Result<Vec<String>> {
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path).await?;
    Ok(df.schema().fields().iter().map(|f| f.name().clone()).collect())
}

/// Sorted distinct stringified values of one column — used by the Intent layer
/// to discover the class list (and count) for a classification task on a
/// tabular or text file.
pub async fn distinct_string_values(path: &str, column: &str) -> Result<Vec<String>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path).await?;
    let columns: Vec<String> = df.schema().fields().iter().map(|f| f.name().clone()).collect();
    let idx = columns
        .iter()
        .position(|c| c == column)
        .ok_or_else(|| cfg(format!("column `{column}` not found (columns: {})", columns.join(", "))))?;

    let batches = df.collect().await.map_err(|e| cfg(e.to_string()))?;
    let opts = FormatOptions::default();
    let mut set = std::collections::BTreeSet::new();
    for batch in &batches {
        let fmt = ArrayFormatter::try_new(batch.column(idx).as_ref(), &opts).map_err(|e| cfg(e.to_string()))?;
        for r in 0..batch.num_rows() {
            set.insert(fmt.value(r).to_string());
        }
    }
    Ok(set.into_iter().collect())
}

/// Count of each distinct value in one column (`(value, count)`, sorted by
/// value) — used by the diagnostics layer to check class balance for a
/// tabular/text classification target.
pub async fn label_counts(path: &str, column: &str) -> Result<Vec<(String, usize)>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path).await?;
    let columns: Vec<String> = df.schema().fields().iter().map(|f| f.name().clone()).collect();
    let idx = columns
        .iter()
        .position(|c| c == column)
        .ok_or_else(|| cfg(format!("column `{column}` not found (columns: {})", columns.join(", "))))?;

    let batches = df.collect().await.map_err(|e| cfg(e.to_string()))?;
    let opts = FormatOptions::default();
    let mut counts: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    for batch in &batches {
        let fmt = ArrayFormatter::try_new(batch.column(idx).as_ref(), &opts).map_err(|e| cfg(e.to_string()))?;
        for r in 0..batch.num_rows() {
            *counts.entry(fmt.value(r).to_string()).or_insert(0) += 1;
        }
    }
    Ok(counts.into_iter().collect())
}

/// Every column that is fully numeric, as `(name, values)` — a column is
/// included only if *every* cell parses as a number. Used by the diagnostics
/// layer's feature/target leakage check (correlation needs numeric columns).
pub async fn numeric_columns(path: &str) -> Result<Vec<(String, Vec<f64>)>> {
    let cfg = |msg: String| BrainBuilderError::ConfigError(msg);
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path).await?;
    let columns: Vec<String> = df.schema().fields().iter().map(|f| f.name().clone()).collect();
    let batches = df.collect().await.map_err(|e| cfg(e.to_string()))?;
    let opts = FormatOptions::default();

    let mut result: Vec<(String, Vec<f64>)> = Vec::new();
    for (idx, name) in columns.iter().enumerate() {
        let mut values = Vec::new();
        let mut all_numeric = true;
        for batch in &batches {
            let fmt = ArrayFormatter::try_new(batch.column(idx).as_ref(), &opts).map_err(|e| cfg(e.to_string()))?;
            for r in 0..batch.num_rows() {
                match fmt.value(r).to_string().trim().parse::<f64>() {
                    Ok(v) => values.push(v),
                    Err(_) => {
                        all_numeric = false;
                        break;
                    }
                }
            }
            if !all_numeric {
                break;
            }
        }
        if all_numeric && !values.is_empty() {
            result.push((name.clone(), values));
        }
    }
    Ok(result)
}

/// Column names + the first `limit` rows (cells stringified for display) of a
/// dataset file — powers the GUI's dataset preview table.
#[derive(Debug, Clone, serde::Serialize)]
pub struct DatasetPreview {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<String>>,
}

pub async fn preview_dataset(path: &str, limit: usize) -> Result<DatasetPreview> {
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path)
        .await?
        .limit(0, Some(limit))
        .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;

    let columns: Vec<String> = df
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    let batches = df.collect().await.map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
    let opts = FormatOptions::default();
    let mut rows = Vec::new();
    for batch in &batches {
        let formatters: Vec<ArrayFormatter> = batch
            .columns()
            .iter()
            .map(|c| ArrayFormatter::try_new(c.as_ref(), &opts))
            .collect::<std::result::Result<_, _>>()
            .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
        for row in 0..batch.num_rows() {
            rows.push(formatters.iter().map(|f| f.value(row).to_string()).collect());
            if rows.len() >= limit {
                break;
            }
        }
    }
    Ok(DatasetPreview { columns, rows })
}

/// Load the first `limit` rows of a dataset file as a single `RecordBatch` —
/// used for one-off inference (Predict UI), as opposed to `load_dataset`'s
/// full streaming iterator for training.
pub async fn load_batch(path: &str, limit: usize) -> Result<RecordBatch> {
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path)
        .await?
        .limit(0, Some(limit))
        .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
    let batches = df.collect().await.map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
    let schema = batches
        .first()
        .map(|b| b.schema())
        .ok_or_else(|| BrainBuilderError::ConfigError(format!("`{path}` contains no rows to load")))?;
    arrow::compute::concat_batches(&schema, &batches)
        .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
}

/// Loads an entire dataset file (csv/parquet) as a streaming iterator over
/// however many chunks DataFusion partitions it into, with no row cap — used
/// by batch prediction, which (unlike `load_batch`'s preview-sized limit)
/// must run over every row. No preprocessing is applied, matching `predict`'s
/// existing behavior for the same dataset.
pub async fn load_all(path: &str) -> Result<Box<dyn DataIterator>> {
    let ctx = SessionContext::new();
    let df = read_file(&ctx, path).await?;
    let batches = df.collect().await.map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
    Ok(Box::new(InMemoryIterator { batches, index: 0 }))
}

/// NOTE: this is async (DataFusion's `collect()` is), so callers must
/// `.await` it — the original blueprint called this from a non-async fn,
/// which would not compile.
pub async fn load_dataset(config: &crate::bbir::TrainingConfig) -> Result<Box<dyn DataIterator>> {
    if config.data_source.source_type == "text_sequence" {
        return load_text_sequence_dataset(&config.data_source);
    }
    if config.data_source.source_type == "image_folder" {
        return load_image_folder_dataset(&config.data_source);
    }
    if config.data_source.source_type == "text_column" {
        return load_text_column_dataset(&config.data_source).await;
    }
    let ctx = SessionContext::new();
    let df = match config.data_source.source_type.as_str() {
        "file" => read_file(&ctx, &config.data_source.path_or_uri).await?,
        other => return Err(BrainBuilderError::ConfigError(format!("unsupported data source: {other}"))),
    };
    let batches = df.collect().await.map_err(|e| BrainBuilderError::ConfigError(e.to_string()))?;
    let batches = batches
        .into_iter()
        .map(|batch| crate::data::etl::apply_steps(batch, &config.data_source.preprocessing))
        .collect::<Result<Vec<_>>>()?;
    Ok(Box::new(InMemoryIterator { batches, index: 0 }))
}

/// Tokenizes a real text file into a real word-level vocabulary (see
/// `data::text`), windows it into fixed-length `(context, next_token)`
/// training examples, and packs each `batch_size`-sized group of windows
/// into a `RecordBatch` whose columns are `tok0..tok{seq_len-1}, target` —
/// the exact shape `ExecutionPlan::bind_data_ports` already knows how to
/// stack into a real `(batch, seq)` matrix for an `embedding` node's `ids`
/// port (see `runtime::scheduler`), so no new tensor plumbing is needed
/// beyond this dataset-loading step. `preprocessing` steps are ignored here
/// (they're aimed at numeric tabular columns; token id columns aren't a
/// meaningful target for normalize/cast).
fn load_text_sequence_dataset(config: &crate::bbir::DataSourceConfig) -> Result<Box<dyn DataIterator>> {
    let text = std::fs::read_to_string(&config.path_or_uri).map_err(|e| {
        BrainBuilderError::ConfigError(format!("failed to read text dataset `{}`: {e}", config.path_or_uri))
    })?;
    let seq_len = config.sequence_length.ok_or_else(|| {
        BrainBuilderError::ConfigError("text_sequence source requires `sequence_length` to be set".into())
    })?;
    let vocab_size = config.vocab_size.unwrap_or(10_000);

    let vocab = crate::data::text::Vocab::build(&text, vocab_size);
    let ids = vocab.encode(&text);
    let windows = crate::data::text::windows(&ids, seq_len)?;

    let batch_size = config.batch_size.max(1);
    let batches = windows
        .chunks(batch_size)
        .map(|chunk| windows_to_record_batch(chunk, seq_len))
        .collect::<Result<Vec<_>>>()?;
    Ok(Box::new(InMemoryIterator { batches, index: 0 }))
}

/// Ingest a folder-of-class-subfolders of images (see `data::vision`) into the
/// standard `px0..px{F-1}, target` batch shape the trainer already consumes.
/// The per-image feature count is derived from the configured `image_size` /
/// `grayscale`; the downstream classifier's `in_features` must match it (the
/// Intent layer wires this automatically — see `intent`).
///
/// Augmentation reuses the generic `preprocessing` step list rather than a
/// dedicated `DataSourceConfig` field: an `augment_flip` step (with no
/// params) turns on `ImageLayout::augment`. This is the one preprocessing op
/// image folders honor — `etl::apply_steps` (normalize/cast) never runs for
/// this source type, since those operate on named tabular columns that don't
/// exist until after decoding.
fn load_image_folder_dataset(config: &crate::bbir::DataSourceConfig) -> Result<Box<dyn DataIterator>> {
    let augment = config.preprocessing.iter().any(|s| s.op == "augment_flip");
    let layout = crate::data::vision::ImageLayout {
        size: config.image_size.unwrap_or(32) as u32,
        grayscale: config.grayscale.unwrap_or(false),
        augment,
    };
    let root = std::path::Path::new(&config.path_or_uri);
    let dataset = crate::data::vision::load_image_folder(root, layout, config.batch_size.max(1))?;
    Ok(Box::new(InMemoryIterator { batches: dataset.batches, index: 0 }))
}

/// Ingest a tabular file (csv/parquet) where one column is free text and
/// another is the class label, into a bag-of-words `(batch, vocab), target`
/// dataset (see `data::tabular_text`). Column values are stringified through
/// the same Arrow `ArrayFormatter` the preview uses, so the text and label
/// columns work regardless of the inferred Arrow type (Utf8, Int64, ...).
async fn load_text_column_dataset(config: &crate::bbir::DataSourceConfig) -> Result<Box<dyn DataIterator>> {
    let rows = read_text_label_rows(config).await?;
    let vocab_size = config.vocab_size.unwrap_or(2_000);
    let dataset = crate::data::tabular_text::build_bag_of_words(&rows, vocab_size)?;

    let batch_size = config.batch_size.max(1);
    let batches = dataset
        .rows
        .chunks(batch_size)
        .map(|chunk| bow_rows_to_record_batch(chunk, dataset.feature_count))
        .collect::<Result<Vec<_>>>()?;
    Ok(Box::new(InMemoryIterator { batches, index: 0 }))
}

fn bow_rows_to_record_batch(chunk: &[(Vec<f32>, f32)], feature_count: usize) -> Result<RecordBatch> {
    use arrow::array::Float32Array;
    use arrow::datatypes::{DataType as ArrowDataType, Field, Schema};
    use std::sync::Arc;

    let mut columns: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk.len()); feature_count + 1];
    for (features, label) in chunk {
        for (i, value) in features.iter().enumerate() {
            columns[i].push(*value);
        }
        columns[feature_count].push(*label);
    }
    let fields: Vec<Field> = (0..feature_count)
        .map(|i| Field::new(format!("feat{i}"), ArrowDataType::Float32, false))
        .chain(std::iter::once(Field::new("target", ArrowDataType::Float32, false)))
        .collect();
    let schema = Arc::new(Schema::new(fields));
    let arrays: Vec<Arc<dyn arrow::array::Array>> = columns
        .into_iter()
        .map(|c| Arc::new(Float32Array::from(c)) as Arc<dyn arrow::array::Array>)
        .collect();
    RecordBatch::try_new(schema, arrays).map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
}

fn windows_to_record_batch(chunk: &[(Vec<u32>, u32)], seq_len: usize) -> Result<RecordBatch> {
    use arrow::array::Float32Array;
    use arrow::datatypes::{DataType as ArrowDataType, Field, Schema};
    use std::sync::Arc;

    // Ids are stored as float32 like every other tensor in this system (see
    // interop/python.rs's module doc) — safe up to 2^24 distinct ids, far
    // beyond any realistic vocabulary size.
    let mut columns: Vec<Vec<f32>> = vec![Vec::with_capacity(chunk.len()); seq_len + 1];
    for (window, target) in chunk {
        for (i, id) in window.iter().enumerate() {
            columns[i].push(*id as f32);
        }
        columns[seq_len].push(*target as f32);
    }

    let fields: Vec<Field> = (0..seq_len)
        .map(|i| Field::new(format!("tok{i}"), ArrowDataType::Float32, false))
        .chain(std::iter::once(Field::new("target", ArrowDataType::Float32, false)))
        .collect();
    let schema = Arc::new(Schema::new(fields));
    let arrays: Vec<Arc<dyn arrow::array::Array>> =
        columns.into_iter().map(|c| Arc::new(Float32Array::from(c)) as Arc<dyn arrow::array::Array>).collect();
    RecordBatch::try_new(schema, arrays).map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
}

struct InMemoryIterator {
    batches: Vec<RecordBatch>,
    index: usize,
}

impl DataIterator for InMemoryIterator {
    fn next(&mut self) -> Result<Option<RecordBatch>> {
        if self.index < self.batches.len() {
            let batch = self.batches[self.index].clone();
            self.index += 1;
            Ok(Some(batch))
        } else {
            Ok(None)
        }
    }

    fn reset(&mut self) -> Result<()> {
        self.index = 0;
        Ok(())
    }
}
