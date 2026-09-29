// Real permutation-importance-style feature interpretability: for each input
// column, decouple it from its row (see `rotate_column`) and measure how much
// predictions change. A column whose rotation barely moves the output is one
// the model isn't relying on; a column whose rotation swings predictions a
// lot is one the model leans on heavily. This is the same idea behind
// scikit-learn's `permutation_importance`, just deterministic — see
// `rotate_column`'s doc for why a rotation instead of a random shuffle.
use crate::bbir::BBIRGraph;
use crate::interop::protocol::BrainBuilderError;
use crate::orchestrator::Orchestrator;
use crate::Result;
use arrow::array::{ArrayRef, UInt32Array};
use arrow::compute::take;
use arrow::record_batch::RecordBatch;

#[derive(Debug, Clone, serde::Serialize)]
pub struct FeatureImportance {
    pub column: String,
    pub importance: f32,
}

/// Ranks every input feature column (every column except the last, which is
/// the target by this codebase's established convention) by how much
/// predictions move when that column is decoupled from its rows. Returns an
/// empty list for a batch with fewer than 2 columns (no feature columns
/// beyond the target).
pub fn feature_importance(
    orchestrator: &Orchestrator,
    graph: &BBIRGraph,
    batch: RecordBatch,
) -> Result<Vec<FeatureImportance>> {
    let num_cols = batch.num_columns();
    if num_cols < 2 {
        return Ok(Vec::new());
    }

    // `predict` (unlike training's `train_step`) doesn't drop the target
    // column itself — it feeds every column in the batch forward — so the
    // target must be excluded here, matching the count of feature columns
    // the model was actually trained against.
    let feature_names: Vec<String> =
        (0..num_cols - 1).map(|i| batch.schema().field(i).name().clone()).collect();
    let features_only = drop_last_column(&batch)?;

    let baseline = predict_flat(orchestrator, graph, features_only.clone())?;

    let mut scores = Vec::with_capacity(feature_names.len());
    for (col_idx, name) in feature_names.into_iter().enumerate() {
        let rotated = rotate_column(&features_only, col_idx)?;
        let perturbed = predict_flat(orchestrator, graph, rotated)?;
        scores.push(FeatureImportance { column: name, importance: mean_abs_diff(&baseline, &perturbed) });
    }
    scores.sort_by(|a, b| b.importance.partial_cmp(&a.importance).unwrap_or(std::cmp::Ordering::Equal));
    Ok(scores)
}

/// Drops the last column (the target, by convention) from a batch — `predict`
/// expects exactly the feature columns the model was trained on, not the
/// target it was trained to predict.
fn drop_last_column(batch: &RecordBatch) -> Result<RecordBatch> {
    let num_cols = batch.num_columns();
    let schema = batch.schema();
    let fields: Vec<arrow::datatypes::Field> =
        schema.fields().iter().take(num_cols - 1).map(|f| f.as_ref().clone()).collect();
    let columns: Vec<ArrayRef> = batch.columns()[..num_cols - 1].to_vec();
    RecordBatch::try_new(std::sync::Arc::new(arrow::datatypes::Schema::new(fields)), columns)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to drop target column: {e}")))
}

/// The graph's real per-row prediction, flattened. Used to be identified by
/// guessing which of `predict`'s tensors was "the" output from its shape
/// (the smallest tensor whose element count divides evenly by the row
/// count) — that guess is unsound: an echoed multi-column input can be
/// *narrower* than the real output (a sequence model's context window vs. a
/// wide vocabulary head), and any model with more than one value per row (a
/// classifier's `[batch, num_classes]` output) would silently return the
/// wrong tensor entirely under the old heuristic. `Orchestrator::predict_output`
/// resolves this unambiguously via the graph's own declared output port
/// instead of guessing.
pub(crate) fn predict_flat(orchestrator: &Orchestrator, graph: &BBIRGraph, batch: RecordBatch) -> Result<Vec<f32>> {
    let tensor = orchestrator.predict_output(graph.clone(), batch)?;
    let (_, values) = crate::interop::dlpack_support::tensor_to_vec_f32(&tensor)?;
    Ok(values)
}

fn mean_abs_diff(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b.iter()).map(|(x, y)| (x - y).abs()).sum::<f32>() / a.len() as f32
}

/// Rebuilds `batch` with column `col_idx` cyclically rotated by one row —
/// every value that was in the column stays in the column (so its own
/// distribution is untouched), but which row it lands on changes, breaking
/// whatever relationship that column had with the target for this run.
///
/// A true random shuffle is the textbook version of this technique, but it
/// would need a `rand` dependency this crate doesn't otherwise have and would
/// give a different (if statistically similar) answer on every call. A
/// one-row rotation gives the same "does this column matter" signal — full
/// decoupling from row order, values unchanged — deterministically, so a
/// user sees the same ranking every time they ask.
fn rotate_column(batch: &RecordBatch, col_idx: usize) -> Result<RecordBatch> {
    let n = batch.num_rows();
    if n < 2 {
        return Ok(batch.clone());
    }
    let indices: UInt32Array = (0..n).map(|i| Some(((i + 1) % n) as u32)).collect();
    let rotated: ArrayRef = take(batch.column(col_idx), &indices, None)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to rotate column for importance scoring: {e}")))?;

    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns[col_idx] = rotated;
    RecordBatch::try_new(batch.schema(), columns)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to rebuild rotated batch: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::Float32Array;
    use arrow::datatypes::{DataType, Field, Schema};
    use std::sync::Arc;

    fn batch_with_columns(cols: &[(&str, Vec<f32>)]) -> RecordBatch {
        let fields: Vec<Field> = cols.iter().map(|(name, _)| Field::new(*name, DataType::Float32, false)).collect();
        let arrays: Vec<ArrayRef> =
            cols.iter().map(|(_, values)| Arc::new(Float32Array::from(values.clone())) as ArrayRef).collect();
        RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays).unwrap()
    }

    #[test]
    fn rotate_column_shifts_row_alignment_but_keeps_the_same_values() {
        let batch = batch_with_columns(&[("a", vec![10.0, 20.0, 30.0, 40.0]), ("b", vec![1.0, 2.0, 3.0, 4.0])]);
        let rotated = rotate_column(&batch, 0).unwrap();

        let original = batch.column(0).as_any().downcast_ref::<Float32Array>().unwrap();
        let shifted = rotated.column(0).as_any().downcast_ref::<Float32Array>().unwrap();

        // Same multiset of values...
        let mut orig_sorted: Vec<f32> = original.values().to_vec();
        let mut shifted_sorted: Vec<f32> = shifted.values().to_vec();
        orig_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        shifted_sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(orig_sorted, shifted_sorted);

        // ...but shifted by exactly one row: row i now holds what used to be
        // at row i+1.
        for i in 0..original.len() {
            assert_eq!(shifted.value(i), original.value((i + 1) % original.len()));
        }

        // The other column is untouched.
        let untouched = rotated.column(1).as_any().downcast_ref::<Float32Array>().unwrap();
        assert_eq!(untouched.values(), batch.column(1).as_any().downcast_ref::<Float32Array>().unwrap().values());
    }

    #[test]
    fn rotate_column_is_a_noop_below_two_rows() {
        let batch = batch_with_columns(&[("a", vec![10.0]), ("b", vec![1.0])]);
        let rotated = rotate_column(&batch, 0).unwrap();
        let arr = rotated.column(0).as_any().downcast_ref::<Float32Array>().unwrap();
        assert_eq!(arr.value(0), 10.0);
    }

    #[test]
    fn mean_abs_diff_of_identical_vectors_is_zero() {
        assert_eq!(mean_abs_diff(&[1.0, 2.0, 3.0], &[1.0, 2.0, 3.0]), 0.0);
    }

    #[test]
    fn mean_abs_diff_averages_the_absolute_differences() {
        assert!((mean_abs_diff(&[0.0, 0.0], &[1.0, 3.0]) - 2.0).abs() < 1e-6);
    }
}
