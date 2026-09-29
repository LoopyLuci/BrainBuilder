use crate::Tensor;
use crate::Result;

/// Abstract device interface for hardware backends.
pub trait Device: Send + Sync {
    fn name(&self) -> &str;
    fn alloc(&self, shape: &[usize], dtype: crate::component::descriptor::DataType) -> Tensor;
    fn exec(&self, kernel: &str, inputs: &[&Tensor]) -> Result<Tensor>;
    fn sync(&self) -> Result<()>;
}
