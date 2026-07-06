// Converts an Arrow `RecordBatch` (what `data::source::load_dataset` yields)
// into zero-copy CPU DLPack tensors, one per column. Each column becomes a
// 1-D tensor of length `num_rows` — this is the honest mapping for
// arbitrary tabular data; component descriptors that expect multi-dim
// tensors are expected to reshape via their own preprocessing, not here.
use crate::interop::arena::SharedArena;
use crate::interop::dlpack_support::cpu_context;
use crate::interop::protocol::BrainBuilderError;
use crate::{Result, Tensor};
use arrow::array::{BooleanArray, Float32Array, Float64Array, Int32Array, Int64Array};
use arrow::datatypes::DataType as ArrowDataType;
use arrow::record_batch::RecordBatch;

pub fn record_batch_to_tensors(batch: &RecordBatch, arena: &SharedArena) -> Result<Vec<Tensor>> {
    (0..batch.num_columns())
        .map(|i| column_to_tensor(batch, i, arena))
        .collect()
}

/// Stacks N per-column 1-D (length = num_rows) tensors into one real
/// `(num_rows, N)` 2-D matrix, row-major — what `linear`/`conv2d`-style
/// components actually need (their `input` port is a proper feature matrix,
/// not N independent length-`batch` vectors). Elementwise components like
/// `scale` don't need this; callers only invoke it when a component declares
/// a single multi-feature data port for what would otherwise be several
/// dataset columns.
pub fn stack_columns_into_matrix(columns: &[Tensor], arena: &SharedArena) -> Result<Tensor> {
    let num_cols = columns.len();
    let mut per_column = Vec::with_capacity(num_cols);
    let mut num_rows = 0usize;
    for col in columns {
        let (_, values) = crate::interop::dlpack_support::tensor_to_vec_f32(col)?;
        num_rows = values.len();
        per_column.push(values);
    }

    let mut matrix = vec![0f32; num_rows * num_cols];
    for (c, values) in per_column.iter().enumerate() {
        for (r, v) in values.iter().enumerate() {
            matrix[r * num_cols + c] = *v;
        }
    }

    let tensor = arena.allocate(
        &[num_rows as i64, num_cols as i64],
        dlpack::DataType { code: dlpack::data_type_codes::FLOAT, bits: 32, lanes: 1 },
        cpu_context(),
    );
    unsafe {
        let dst = (*tensor.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(matrix.as_ptr(), dst, matrix.len());
    }
    Ok(tensor)
}

fn column_to_tensor(batch: &RecordBatch, index: usize, arena: &SharedArena) -> Result<Tensor> {
    let column = batch.column(index);
    let field = batch.schema().field(index).clone();

    if column.null_count() > 0 {
        return Err(BrainBuilderError::TypeMismatch(format!(
            "column `{}` has null values — nulls aren't representable in a dense DLPack tensor \
             yet (needs an explicit fill/drop preprocessing step first)",
            field.name()
        )));
    }

    match field.data_type() {
        ArrowDataType::Float32 => {
            let array = column
                .as_any()
                .downcast_ref::<Float32Array>()
                .expect("DataType::Float32 field backed by a non-Float32Array");
            copy_into_tensor(array.values(), dlpack::DataType {
                code: dlpack::data_type_codes::FLOAT,
                bits: 32,
                lanes: 1,
            }, arena)
        }
        ArrowDataType::Float64 => {
            // DataFusion/Arrow's CSV schema inference defaults numeric
            // columns to Float64 — this is the common case for real user
            // CSVs, not an edge case. Every ML component here works in
            // float32, so narrow (same as `torch.tensor(x, dtype=float32)`
            // would do in any real pipeline) rather than reject the column.
            let array = column
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("DataType::Float64 field backed by a non-Float64Array");
            let narrowed: Vec<f32> = array.values().iter().map(|&v| v as f32).collect();
            copy_into_tensor(&narrowed, dlpack::DataType {
                code: dlpack::data_type_codes::FLOAT,
                bits: 32,
                lanes: 1,
            }, arena)
        }
        ArrowDataType::Int32 => {
            let array = column
                .as_any()
                .downcast_ref::<Int32Array>()
                .expect("DataType::Int32 field backed by a non-Int32Array");
            copy_into_tensor(array.values(), dlpack::DataType {
                code: dlpack::data_type_codes::INT,
                bits: 32,
                lanes: 1,
            }, arena)
        }
        ArrowDataType::Int64 => {
            let array = column
                .as_any()
                .downcast_ref::<Int64Array>()
                .expect("DataType::Int64 field backed by a non-Int64Array");
            copy_into_tensor(array.values(), dlpack::DataType {
                code: dlpack::data_type_codes::INT,
                bits: 64,
                lanes: 1,
            }, arena)
        }
        ArrowDataType::Boolean => {
            let array = column
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("DataType::Boolean field backed by a non-BooleanArray");
            // BooleanArray is bit-packed; DLPack has no bit-packed bool
            // dtype, so expand to one byte (0/1) per element.
            let bytes: Vec<u8> = array.values().iter().map(u8::from).collect();
            copy_into_tensor(
                &bytes,
                dlpack::DataType {
                    code: dlpack::data_type_codes::UINT,
                    bits: 8,
                    lanes: 1,
                },
                arena,
            )
        }
        other => Err(BrainBuilderError::TypeMismatch(format!(
            "column `{}` has arrow type {other:?}, which has no DLPack tensor mapping yet",
            field.name()
        ))),
    }
}

fn copy_into_tensor<T: Copy>(values: &[T], dtype: dlpack::DataType, arena: &SharedArena) -> Result<Tensor> {
    let tensor = arena.allocate(&[values.len() as i64], dtype, cpu_context());
    unsafe {
        let dst = (*tensor.0).dl_tensor.data as *mut T;
        std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
    }
    Ok(tensor)
}

#[cfg(test)]
mod tests {
    use super::*;
    use arrow::array::ArrayRef;
    use arrow::datatypes::{Field, Schema};
    use std::sync::Arc;

    #[test]
    fn converts_float32_column_to_tensor() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", ArrowDataType::Float32, false)]));
        let array: ArrayRef = Arc::new(Float32Array::from(vec![1.0, 2.0, 3.0]));
        let batch = RecordBatch::try_new(schema, vec![array]).unwrap();

        let arena = SharedArena::new();
        let tensors = record_batch_to_tensors(&batch, &arena).unwrap();

        assert_eq!(tensors.len(), 1);
        unsafe {
            let t = &(*tensors[0].0).dl_tensor;
            assert_eq!(t.ndim, 1);
            assert_eq!(*t.shape, 3);
            let values = std::slice::from_raw_parts(t.data as *const f32, 3);
            assert_eq!(values, &[1.0, 2.0, 3.0]);
        }
    }

    #[test]
    fn converts_float64_column_narrowing_to_float32() {
        // Regression test: DataFusion's CSV reader infers plain decimal
        // columns as Float64 by default — this is what the bundled first-run
        // example's dataset actually produces, and the original code path
        // only handled Float32/Int32/Int64/Boolean, erroring on every real
        // CSV with decimal values.
        let schema = Arc::new(Schema::new(vec![Field::new("x", ArrowDataType::Float64, false)]));
        let array: ArrayRef = Arc::new(arrow::array::Float64Array::from(vec![1.5, 2.5, 3.5]));
        let batch = RecordBatch::try_new(schema, vec![array]).unwrap();

        let arena = SharedArena::new();
        let tensors = record_batch_to_tensors(&batch, &arena).unwrap();

        unsafe {
            let t = &(*tensors[0].0).dl_tensor;
            let values = std::slice::from_raw_parts(t.data as *const f32, 3);
            assert_eq!(values, &[1.5f32, 2.5, 3.5]);
        }
    }

    #[test]
    fn stacks_columns_into_a_real_batch_by_features_matrix() {
        let arena = SharedArena::new();
        let a = arena.allocate(&[3], dlpack::DataType { code: dlpack::data_type_codes::FLOAT, bits: 32, lanes: 1 }, cpu_context());
        let b = arena.allocate(&[3], dlpack::DataType { code: dlpack::data_type_codes::FLOAT, bits: 32, lanes: 1 }, cpu_context());
        unsafe {
            std::ptr::copy_nonoverlapping([1.0f32, 2.0, 3.0].as_ptr(), (*a.0).dl_tensor.data as *mut f32, 3);
            std::ptr::copy_nonoverlapping([10.0f32, 20.0, 30.0].as_ptr(), (*b.0).dl_tensor.data as *mut f32, 3);
        }
        let matrix = stack_columns_into_matrix(&[a, b], &arena).unwrap();
        unsafe {
            let t = &(*matrix.0).dl_tensor;
            assert_eq!((t.ndim, *t.shape, *t.shape.offset(1)), (2, 3, 2));
            let values = std::slice::from_raw_parts(t.data as *const f32, 6);
            assert_eq!(values, &[1.0, 10.0, 2.0, 20.0, 3.0, 30.0]);
        }
    }

    #[test]
    fn rejects_columns_with_nulls() {
        let schema = Arc::new(Schema::new(vec![Field::new("x", ArrowDataType::Float32, true)]));
        let array: ArrayRef = Arc::new(Float32Array::from(vec![Some(1.0), None, Some(3.0)]));
        let batch = RecordBatch::try_new(schema, vec![array]).unwrap();

        let arena = SharedArena::new();
        assert!(record_batch_to_tensors(&batch, &arena).is_err());
    }
}
