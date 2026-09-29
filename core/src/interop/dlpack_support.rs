// Real `dlpack` 0.2 is a bare `#[repr(C)]` binding to the DLPack C ABI —
// there's no safe constructor, so allocation/deallocation is hand-rolled here
// once and shared by every backend that needs a CPU-resident DLPack tensor.
use crate::component::descriptor::DataType;
use std::alloc::Layout;
use std::os::raw::c_void;

/// Maps our component-descriptor `DataType` to the DLPack `DataType` triple
/// (type-code, bit-width, lanes). Shared by every backend that allocates
/// DLPack tensors (`CpuDevice::alloc`, the Arrow -> tensor bridge) so the
/// mapping is defined exactly once.
pub fn dtype_to_dlpack(dtype: DataType) -> dlpack::DataType {
    match dtype {
        DataType::Float32 => dlpack::DataType {
            code: dlpack::data_type_codes::FLOAT,
            bits: 32,
            lanes: 1,
        },
        DataType::Float16 => dlpack::DataType {
            code: dlpack::data_type_codes::FLOAT,
            bits: 16,
            lanes: 1,
        },
        DataType::Int32 => dlpack::DataType {
            code: dlpack::data_type_codes::INT,
            bits: 32,
            lanes: 1,
        },
        DataType::Int64 => dlpack::DataType {
            code: dlpack::data_type_codes::INT,
            bits: 64,
            lanes: 1,
        },
        DataType::Bool => dlpack::DataType {
            code: dlpack::data_type_codes::UINT,
            bits: 8,
            lanes: 1,
        },
    }
}

/// The `Context` (device descriptor) for a CPU-resident tensor.
pub fn cpu_context() -> dlpack::Context {
    dlpack::Context {
        device_type: dlpack::device_type_codes::CPU,
        device_id: 0,
    }
}

/// Reads a CPU-resident float32 `Tensor`'s shape and values out into plain
/// Rust types — for boundaries that need to hand tensor data somewhere that
/// isn't DLPack-aware (e.g. serializing a prediction result to JSON for the
/// GUI). Errors on non-float32 tensors rather than silently reinterpreting
/// bytes.
pub fn tensor_to_vec_f32(tensor: &crate::Tensor) -> crate::Result<(Vec<i64>, Vec<f32>)> {
    unsafe {
        let t = &(*tensor.0).dl_tensor;
        if t.dtype.code != dlpack::data_type_codes::FLOAT || t.dtype.bits != 32 {
            return Err(crate::interop::protocol::BrainBuilderError::TypeMismatch(
                "tensor_to_vec_f32 requires a float32 tensor".into(),
            ));
        }
        let shape: Vec<i64> = (0..t.ndim as isize).map(|i| *t.shape.offset(i)).collect();
        let len = shape.iter().product::<i64>().max(0) as usize;
        let values = std::slice::from_raw_parts(t.data as *const f32, len).to_vec();
        Ok((shape, values))
    }
}

/// The inverse of `tensor_to_vec_f32`: allocates a real arena-backed CPU
/// float32 tensor and copies `values` into it. Used at the boundary between
/// plain-Rust tensor representations (e.g. `TensorPayload` on the cluster
/// wire protocol) and DLPack-aware code (`PythonBridge`, `ExecutionPlan`).
pub fn tensor_from_vec_f32(
    arena: &crate::interop::arena::SharedArena,
    shape: &[i64],
    values: &[f32],
) -> crate::Tensor {
    let tensor = arena.allocate(shape, dtype_to_dlpack(DataType::Float32), cpu_context());
    unsafe {
        let dst = (*tensor.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
    }
    tensor
}

/// Stashed alongside the tensor via `manager_ctx` so the deleter can free
/// exactly what was allocated: the data buffer (needs its `Layout` back) and
/// the `shape` array (owned by the `Vec`, freed when this drops).
struct ManagerCtx {
    data_layout: Layout,
    #[allow(dead_code)] // kept alive only to free the backing allocation on drop
    shape: Vec<i64>,
}

// DLPack's contract for `deleter` is that it frees *everything*: the data
// buffer, any side allocations (the shape array here), and the
// `ManagedTensor` struct's own backing memory. We heap-allocate that struct
// via `Box`, so reclaiming it with `Box::from_raw` is what frees it; the
// data buffer and shape `Vec` are freed via the stashed `ManagerCtx`.
extern "C" fn managed_tensor_deleter(tensor: *mut dlpack::ManagedTensor) {
    unsafe {
        let boxed = Box::from_raw(tensor);
        let manager_ctx = Box::from_raw(boxed.manager_ctx as *mut ManagerCtx);
        std::alloc::dealloc(boxed.dl_tensor.data as *mut u8, manager_ctx.data_layout);
    }
}

/// Allocate a zeroed CPU buffer and wrap it in a heap-allocated
/// `dlpack::ManagedTensor` whose deleter frees the buffer, the shape array,
/// and the `ManagedTensor` allocation itself.
pub fn alloc_managed_tensor(
    shape: &[i64],
    dtype: dlpack::DataType,
    ctx: dlpack::Context,
) -> *mut dlpack::ManagedTensor {
    let elems: i64 = shape.iter().product::<i64>().max(1);
    let byte_size = (elems as usize * dtype.bits as usize / 8).max(1);
    let layout = Layout::from_size_align(byte_size, 64).unwrap();
    let data = unsafe { std::alloc::alloc_zeroed(layout) };

    let mut shape_vec = shape.to_vec();
    // Safe: taking this pointer before moving `shape_vec` into the Box is
    // fine because moving a Vec only moves its (ptr, len, cap) triple — the
    // heap buffer it points at doesn't move.
    let shape_ptr = shape_vec.as_mut_ptr();
    let ndim = shape_vec.len() as i32;

    let manager_ctx = Box::into_raw(Box::new(ManagerCtx {
        data_layout: layout,
        shape: shape_vec,
    })) as *mut c_void;

    Box::into_raw(Box::new(dlpack::ManagedTensor {
        dl_tensor: dlpack::Tensor {
            data: data as *mut c_void,
            ctx,
            ndim,
            dtype,
            shape: shape_ptr,
            strides: std::ptr::null_mut(),
            byte_offset: 0,
        },
        manager_ctx,
        deleter: managed_tensor_deleter,
    }))
}
