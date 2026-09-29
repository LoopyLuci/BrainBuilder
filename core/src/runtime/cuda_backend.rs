// Gated behind the `gpu` feature. This genuinely cannot be finished or
// verified in this environment: there's no NVIDIA GPU, and `cudarc`'s own
// build script (correctly, per its docs) refuses to configure without a
// real CUDA toolkit (`nvcc`) to detect a version against — so
// `cargo build --features gpu` fails *before rustc ever runs* on this file.
// That means nothing below this point can be compile-checked here; writing
// speculative kernel/allocation code against an API I can't verify would
// just reproduce the original blueprint's mistake in a different crate.
//
// What *is* fixed here, verified directly against cudarc 0.12.1's real
// source (crates.io, not assumed): `CudaContext::new(ordinal) -> Result<Arc<Self>, _>`
// returns an `Arc`, not a bare `Self` — every safe method on it
// (`alloc_zeros`, `htod_sync_copy`, ...) takes `self: &Arc<Self>` as its
// receiver. The original field here was typed as bare `CudaContext`, which
// would not compile even with a GPU present.
use crate::runtime::device::Device;
use crate::Tensor;
use crate::Result;
use crate::component::descriptor::DataType;
use cudarc::driver::CudaContext;
use std::sync::Arc;

pub struct CudaDevice {
    #[allow(dead_code)]
    ctx: Arc<CudaContext>,
}

impl CudaDevice {
    pub fn new() -> Result<Self> {
        let ctx = CudaContext::new(0)
            .map_err(|e| crate::interop::protocol::BrainBuilderError::Hardware(e.to_string()))?;
        Ok(Self { ctx })
    }
}

impl Device for CudaDevice {
    fn name(&self) -> &str {
        "cuda"
    }

    fn alloc(&self, _shape: &[usize], _dtype: DataType) -> Tensor {
        todo!(
            "CUDA allocation: allocate via `self.ctx.alloc_zeros::<f32>(len)`, then wrap the \
             resulting CudaSlice's device pointer in a dlpack::Tensor with \
             device_type=GPU, keeping the CudaSlice (and this Arc<CudaContext>) alive via \
             manager_ctx exactly like dlpack_support::alloc_managed_tensor does for CPU. \
             Blocked on having real GPU hardware to compile and test against."
        )
    }

    fn exec(&self, _kernel: &str, _inputs: &[&Tensor]) -> Result<Tensor> {
        todo!("CUDA kernel execution — blocked on real GPU hardware, see `alloc` above")
    }

    fn sync(&self) -> Result<()> {
        Ok(())
    }
}
