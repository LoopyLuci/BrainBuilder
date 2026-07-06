use crate::interop::dlpack_support::alloc_managed_tensor;
use crate::TensorHandle;
use std::sync::{Arc, Mutex};

/// Shared memory arena for zero-copy tensor exchange. All runtimes use this.
pub struct SharedArena {
    tensors: Mutex<Vec<Arc<TensorHandle>>>,
}

impl SharedArena {
    pub fn new() -> Self {
        Self {
            tensors: Mutex::new(Vec::new()),
        }
    }

    pub fn allocate(
        &self,
        shape: &[i64],
        dtype: dlpack::DataType,
        ctx: dlpack::Context,
    ) -> Arc<TensorHandle> {
        let tensor_ptr = alloc_managed_tensor(shape, dtype, ctx);
        let arc = Arc::new(TensorHandle(tensor_ptr));
        self.tensors.lock().unwrap().push(arc.clone());
        arc
    }
}

impl Default for SharedArena {
    fn default() -> Self {
        Self::new()
    }
}
