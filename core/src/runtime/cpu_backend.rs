use crate::component::descriptor::DataType;
use crate::interop::dlpack_support::{alloc_managed_tensor, cpu_context, dtype_to_dlpack};
use crate::runtime::device::Device;
use crate::runtime::ops;
use crate::Result;
use crate::Tensor;
use crate::TensorHandle;
use std::sync::Arc;

pub struct CpuDevice;

impl Device for CpuDevice {
    fn name(&self) -> &str {
        "cpu"
    }

    fn alloc(&self, shape: &[usize], dtype: DataType) -> Tensor {
        let dl_dtype = dtype_to_dlpack(dtype);
        let shape_i64: Vec<i64> = shape.iter().map(|&x| x as i64).collect();
        let tensor_ptr = alloc_managed_tensor(&shape_i64, dl_dtype, cpu_context());
        Arc::new(TensorHandle(tensor_ptr))
    }

    fn exec(&self, kernel: &str, inputs: &[&Tensor]) -> Result<Tensor> {
        ops::dispatch(kernel, inputs)
    }

    fn sync(&self) -> Result<()> {
        Ok(())
    }
}
