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
    arrow::compute::concat_batches(&batches[0].schema(), &batches)
        .map_err(|e| BrainBuilderError::ConfigError(e.to_string()))
}

/// NOTE: this is async (DataFusion's `collect()` is), so callers must
/// `.await` it — the original blueprint called this from a non-async fn,
/// which would not compile.
pub async fn load_dataset(config: &crate::bbir::TrainingConfig) -> Result<Box<dyn DataIterator>> {
    if config.data_source.source_type == "text_sequence" {
        return load_text_sequence_dataset(&config.data_source);
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
