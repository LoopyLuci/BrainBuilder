// Native-Rust CPU kernel table for `Device::exec`. None of the shipped
// components under `components/*.edn` currently declare `:language "rust"`
// (they're all Python) — so there's no real consumer wiring a specific
// kernel name yet. Rather than inventing kernels to match hypothetical
// components, this registers a small number of genuinely working,
// unit-tested elementwise ops and returns a real error (not a `todo!()`
// panic) for anything unregistered, so the dispatch mechanism itself is
// complete and correct for any input, even though the kernel *catalog* is
// intentionally small until real Rust-language components exist.
use crate::interop::dlpack_support::{alloc_managed_tensor, cpu_context};
use crate::interop::protocol::BrainBuilderError;
use crate::{Result, Tensor, TensorHandle};
use std::sync::Arc;

pub fn dispatch(kernel: &str, inputs: &[&Tensor]) -> Result<Tensor> {
    match kernel {
        "identity" => identity(inputs),
        "relu" => relu(inputs),
        other => Err(BrainBuilderError::UnsupportedLanguage(format!(
            "no native CPU kernel registered for `{other}`"
        ))),
    }
}

fn as_f32_slice(tensor: &Tensor) -> Result<&[f32]> {
    unsafe {
        let t = &(*tensor.0).dl_tensor;
        if t.dtype.code != dlpack::data_type_codes::FLOAT || t.dtype.bits != 32 {
            return Err(BrainBuilderError::TypeMismatch(
                "native CPU kernels currently only support float32 tensors".into(),
            ));
        }
        let len: i64 = (0..t.ndim as isize).map(|i| *t.shape.offset(i)).product();
        Ok(std::slice::from_raw_parts(t.data as *const f32, len.max(0) as usize))
    }
}

fn alloc_like(tensor: &Tensor) -> Tensor {
    unsafe {
        let t = &(*tensor.0).dl_tensor;
        let shape: Vec<i64> = (0..t.ndim as isize).map(|i| *t.shape.offset(i)).collect();
        let ptr = alloc_managed_tensor(&shape, t.dtype, cpu_context());
        Arc::new(TensorHandle(ptr))
    }
}

fn one_input<'a>(inputs: &[&'a Tensor], kernel: &str) -> Result<&'a Tensor> {
    inputs
        .first()
        .copied()
        .ok_or_else(|| BrainBuilderError::ConfigError(format!("{kernel}: expected exactly 1 input")))
}

fn identity(inputs: &[&Tensor]) -> Result<Tensor> {
    let input = one_input(inputs, "identity")?;
    let src = as_f32_slice(input)?;
    let out = alloc_like(input);
    unsafe {
        let dst = (*out.0).dl_tensor.data as *mut f32;
        std::ptr::copy_nonoverlapping(src.as_ptr(), dst, src.len());
    }
    Ok(out)
}

fn relu(inputs: &[&Tensor]) -> Result<Tensor> {
    let input = one_input(inputs, "relu")?;
    let src = as_f32_slice(input)?;
    let out = alloc_like(input);
    unsafe {
        let dst = std::slice::from_raw_parts_mut((*out.0).dl_tensor.data as *mut f32, src.len());
        for (d, s) in dst.iter_mut().zip(src.iter()) {
            *d = s.max(0.0);
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::component::descriptor::DataType;
    use crate::runtime::cpu_backend::CpuDevice;
    use crate::runtime::device::Device;

    fn tensor_from(values: &[f32]) -> Tensor {
        let device = CpuDevice;
        let t = device.alloc(&[values.len()], DataType::Float32);
        unsafe {
            let dst = (*t.0).dl_tensor.data as *mut f32;
            std::ptr::copy_nonoverlapping(values.as_ptr(), dst, values.len());
        }
        t
    }

    fn read(tensor: &Tensor, len: usize) -> Vec<f32> {
        unsafe { std::slice::from_raw_parts((*tensor.0).dl_tensor.data as *const f32, len).to_vec() }
    }

    #[test]
    fn relu_zeroes_negatives() {
        let input = tensor_from(&[-2.0, -0.5, 0.0, 1.5, 3.0]);
        let output = dispatch("relu", &[&input]).unwrap();
        assert_eq!(read(&output, 5), vec![0.0, 0.0, 0.0, 1.5, 3.0]);
    }

    #[test]
    fn identity_copies_values() {
        let input = tensor_from(&[1.0, 2.0, 3.0]);
        let output = dispatch("identity", &[&input]).unwrap();
        assert_eq!(read(&output, 3), vec![1.0, 2.0, 3.0]);
    }

    #[test]
    fn unknown_kernel_errors_instead_of_panicking() {
        let input = tensor_from(&[1.0]);
        let result = dispatch("nonexistent_kernel", &[&input]);
        assert!(result.is_err());
    }
}
