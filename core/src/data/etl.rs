// Real preprocessing-step execution for the `PreprocStep` list on
// `DataSourceConfig`. Two ops are implemented for real against genuine Arrow
// kernels: `normalize` (z-score a float32 column) and `cast` (arrow's own
// `compute::cast`, e.g. int64 -> float32). Image/text ops like "resize" or
// "tokenize" (mentioned as future examples in the original design) would
// need a real image-decode or tokenizer dependency that isn't part of this
// crate yet — rather than fake those, unknown ops return a clear error.
use crate::bbir::PreprocStep;
use crate::interop::protocol::BrainBuilderError;
use crate::Result;
use arrow::array::{ArrayRef, Float32Array};
use arrow::compute::cast;
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use std::sync::Arc;

pub fn apply_steps(batch: RecordBatch, steps: &[PreprocStep]) -> Result<RecordBatch> {
    steps.iter().try_fold(batch, apply_step)
}

fn apply_step(batch: RecordBatch, step: &PreprocStep) -> Result<RecordBatch> {
    match step.op.as_str() {
        "normalize" => normalize(batch, step),
        "cast" => cast_column(batch, step),
        other => Err(BrainBuilderError::ConfigError(format!(
            "unknown preprocessing op `{other}` (supported: normalize, cast)"
        ))),
    }
}

fn column_param(step: &PreprocStep) -> Result<&str> {
    step.params
        .get("column")
        .and_then(|v| v.as_str())
        .ok_or_else(|| {
            BrainBuilderError::ConfigError(format!("preprocessing op `{}` needs a `column` param", step.op))
        })
}

fn column_index(batch: &RecordBatch, name: &str) -> Result<usize> {
    batch
        .schema()
        .index_of(name)
        .map_err(|_| BrainBuilderError::ConfigError(format!("no column named `{name}` in batch")))
}

fn replace_column(batch: RecordBatch, index: usize, new_array: ArrayRef) -> Result<RecordBatch> {
    let mut fields: Vec<Field> = batch.schema().fields().iter().map(|f| (**f).clone()).collect();
    fields[index] = Field::new(fields[index].name(), new_array.data_type().clone(), fields[index].is_nullable());
    let schema = Arc::new(Schema::new(fields));

    let mut columns: Vec<ArrayRef> = batch.columns().to_vec();
    columns[index] = new_array;

    RecordBatch::try_new(schema, columns)
        .map_err(|e| BrainBuilderError::ConfigError(format!("failed to rebuild batch: {e}")))
}

/// Z-score normalization: `(x - mean) / std`. `std == 0` (a constant column)
/// leaves the column unchanged rather than dividing by zero.
fn normalize(batch: RecordBatch, step: &PreprocStep) -> Result<RecordBatch> {
    let name = column_param(step)?;
    let index = column_index(&batch, name)?;
    let array = batch
        .column(index)
        .as_any()
        .downcast_ref::<Float32Array>()
        .ok_or_else(|| {
            BrainBuilderError::TypeMismatch(format!("`normalize` requires a float32 column, got `{name}`"))
        })?;

    let values = array.values();
    let mean = values.iter().sum::<f32>() / values.len().max(1) as f32;
    let variance = values.iter().map(|v| (v - mean).powi(2)).sum::<f32>() / values.len().max(1) as f32;
    let std = variance.sqrt();

    let normalized: Float32Array = if std == 0.0 {
        array.clone()
    } else {
        values.iter().map(|v| (v - mean) / std).collect()
    };

    replace_column(batch, index, Arc::new(normalized))
}

fn cast_column(batch: RecordBatch, step: &PreprocStep) -> Result<RecordBatch> {
    let name = column_param(step)?;
    let index = column_index(&batch, name)?;
    let dtype_str = step
        .params
        .get("dtype")
        .and_then(|v| v.as_str())
        .ok_or_else(|| BrainBuilderError::ConfigError("`cast` needs a `dtype` param".into()))?;

    let dtype = match dtype_str {
        "float32" => DataType::Float32,
        "int32" => DataType::Int32,
        "int64" => DataType::Int64,
        other => {
            return Err(BrainBuilderError::ConfigError(format!(
                "`cast` doesn't support target dtype `{other}` (supported: float32, int32, int64)"
            )))
        }
    };

    let casted = cast(batch.column(index), &dtype)
        .map_err(|e| BrainBuilderError::TypeMismatch(format!("cast of `{name}` to {dtype_str} failed: {e}")))?;

    replace_column(batch, index, casted)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::Int32Array;
    use arrow::datatypes::Field as ArrowField;
    use serde_json::json;

    fn batch_with_f32_column(name: &str, values: Vec<f32>) -> RecordBatch {
        let schema = Arc::new(Schema::new(vec![ArrowField::new(name, DataType::Float32, false)]));
        let array: ArrayRef = Arc::new(Float32Array::from(values));
        RecordBatch::try_new(schema, vec![array]).unwrap()
    }

    #[test]
    fn normalize_produces_zero_mean_unit_variance() {
        let batch = batch_with_f32_column("x", vec![2.0, 4.0, 4.0, 4.0, 5.0, 5.0, 7.0, 9.0]);
        let step = PreprocStep { op: "normalize".into(), params: json!({"column": "x"}) };
        let result = apply_steps(batch, &[step]).unwrap();

        let array = result.column(0).as_any().downcast_ref::<Float32Array>().unwrap();
        let mean: f32 = array.values().iter().sum::<f32>() / array.len() as f32;
        assert!(mean.abs() < 1e-5, "expected ~0 mean, got {mean}");
    }

    #[test]
    fn cast_converts_int32_column_to_float32() {
        let schema = Arc::new(Schema::new(vec![ArrowField::new("x", DataType::Int32, false)]));
        let array: ArrayRef = Arc::new(Int32Array::from(vec![1, 2, 3]));
        let batch = RecordBatch::try_new(schema, vec![array]).unwrap();

        let step = PreprocStep {
            op: "cast".into(),
            params: json!({"column": "x", "dtype": "float32"}),
        };
        let result = apply_steps(batch, &[step]).unwrap();

        assert_eq!(result.schema().field(0).data_type(), &DataType::Float32);
        let array = result.column(0).as_any().downcast_ref::<Float32Array>().unwrap();
        assert_eq!(array.values(), &[1.0, 2.0, 3.0]);
    }

    #[test]
    fn unknown_op_errors() {
        let batch = batch_with_f32_column("x", vec![1.0]);
        let step = PreprocStep { op: "tokenize".into(), params: json!({}) };
        assert!(apply_steps(batch, &[step]).is_err());
    }
}
