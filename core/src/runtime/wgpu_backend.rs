// Real cross-platform GPU compute via wgpu (Vulkan/DX12/Metal under one API)
// — the "runs on whatever GPU this laptop has" story, as opposed to
// `cuda_backend.rs` which only serves NVIDIA hardware.
//
// Scope, stated honestly: tensors here stay CPU-resident (the same
// `dlpack_support::alloc_managed_tensor` allocator as `CpuDevice`) rather
// than living in a `wgpu::Buffer` across calls. Making a GPU-resident buffer
// satisfy the existing DLPack `Tensor` model (a raw host-visible pointer good
// for the tensor's whole lifetime, freed via a plain deleter) would need
// wgpu buffers kept persistently mapped, which conflicts with wgpu's actual
// map/unmap-around-each-submission requirement — a real redesign of the
// tensor memory model, not a quick addition. What's genuinely real here:
// `exec()` uploads the input tensor's bytes to the GPU, runs an actual WGSL
// compute shader, and downloads the result — the arithmetic itself runs on
// the GPU, verified in `core/tests/wgpu_backend.rs` against a real adapter.
use crate::component::descriptor::DataType;
use crate::interop::dlpack_support::{alloc_managed_tensor, cpu_context, dtype_to_dlpack};
use crate::interop::protocol::BrainBuilderError;
use crate::runtime::device::Device;
use crate::{Result, Tensor, TensorHandle};
use std::sync::Arc;
use wgpu::util::DeviceExt;

const RELU_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input_buf: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_buf: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&input_buf)) {
        output_buf[i] = max(input_buf[i], 0.0);
    }
}
"#;

const IDENTITY_SHADER: &str = r#"
@group(0) @binding(0) var<storage, read> input_buf: array<f32>;
@group(0) @binding(1) var<storage, read_write> output_buf: array<f32>;

@compute @workgroup_size(64)
fn main(@builtin(global_invocation_id) gid: vec3<u32>) {
    let i = gid.x;
    if (i < arrayLength(&input_buf)) {
        output_buf[i] = input_buf[i];
    }
}
"#;

pub struct WgpuDevice {
    device: wgpu::Device,
    queue: wgpu::Queue,
    adapter_name: String,
}

/// A GPU wgpu can drive on this machine, as surfaced to the picker UI. `id` is
/// a stable-enough selector (name + backend) the frontend can persist and pass
/// back to bind that specific device — e.g. an AMD RX 7900 XTX over Vulkan vs.
/// the same card over DX12, or an integrated GPU.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GpuAdapterInfo {
    pub name: String,
    pub backend: String,
    pub device_type: String,
}

impl GpuAdapterInfo {
    fn from_info(info: &wgpu::AdapterInfo) -> Self {
        Self {
            name: info.name.clone(),
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
        }
    }
}

/// Enumerate every real GPU adapter wgpu can see across all backends
/// (Vulkan/DX12/Metal/GL). Returns an empty list on a machine with no
/// compatible GPU rather than erroring, so the picker can show "none found"
/// instead of failing. This is the data behind the device picker — on the
/// user's box an AMD RX 7900 XTX shows up here (typically under Vulkan/DX12).
pub fn list_adapters() -> Vec<GpuAdapterInfo> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    instance
        .enumerate_adapters(wgpu::Backends::all())
        .iter()
        .map(|a| GpuAdapterInfo::from_info(&a.get_info()))
        .collect()
}

impl WgpuDevice {
    /// Picks whatever real adapter the OS/driver gives us first (the point
    /// of wgpu — no vendor-specific setup) and blocks synchronously on its
    /// async setup via `pollster`, since `Device::alloc`/`exec` are sync.
    pub fn new() -> Result<Self> {
        Self::with_preferred(None)
    }

    /// Bind a specific adapter by name (substring match, case-insensitive) —
    /// e.g. `Some("7900 XTX")` to force the discrete AMD card. Falls back to
    /// the default high-performance adapter when `preferred` is `None` or no
    /// adapter matches, so a stale/typo'd preference degrades gracefully rather
    /// than failing to get a GPU at all.
    pub fn with_preferred(preferred: Option<&str>) -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let adapter = preferred
            .and_then(|want| {
                let needle = want.to_lowercase();
                instance
                    .enumerate_adapters(wgpu::Backends::all())
                    .into_iter()
                    .find(|a| a.get_info().name.to_lowercase().contains(&needle))
            })
            .or_else(|| {
                pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    ..Default::default()
                }))
            })
            .ok_or_else(|| BrainBuilderError::Hardware("no wgpu-compatible GPU adapter found".into()))?;
        let adapter_name = adapter.get_info().name;

        let (device, queue) = pollster::block_on(adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: Some("brainbuilder-wgpu-device"),
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
            },
            None,
        ))
        .map_err(|e| BrainBuilderError::Hardware(format!("failed to acquire wgpu device: {e}")))?;

        Ok(Self { device, queue, adapter_name })
    }

    pub fn adapter_name(&self) -> &str {
        &self.adapter_name
    }

    fn run_elementwise_shader(&self, shader_src: &str, input: &[f32]) -> Result<Vec<f32>> {
        let byte_len = std::mem::size_of_val(input) as u64;

        let input_buf = self.device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("bb-input"),
            contents: bytemuck_cast_slice(input),
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
        });
        let output_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bb-output"),
            size: byte_len,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        let staging_buf = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("bb-staging"),
            size: byte_len,
            usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let shader = self.device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bb-elementwise"),
            source: wgpu::ShaderSource::Wgsl(shader_src.into()),
        });
        let pipeline = self.device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("bb-pipeline"),
            layout: None,
            module: &shader,
            entry_point: "main",
        });
        let bind_group_layout = pipeline.get_bind_group_layout(0);
        let bind_group = self.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("bb-bind-group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry { binding: 0, resource: input_buf.as_entire_binding() },
                wgpu::BindGroupEntry { binding: 1, resource: output_buf.as_entire_binding() },
            ],
        });

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &bind_group, &[]);
            let workgroups = input.len().div_ceil(64) as u32;
            pass.dispatch_workgroups(workgroups.max(1), 1, 1);
        }
        encoder.copy_buffer_to_buffer(&output_buf, 0, &staging_buf, 0, byte_len);
        self.queue.submit(Some(encoder.finish()));

        let slice = staging_buf.slice(..);
        let (tx, rx) = std::sync::mpsc::channel();
        slice.map_async(wgpu::MapMode::Read, move |res| {
            let _ = tx.send(res);
        });
        self.device.poll(wgpu::Maintain::Wait);
        rx.recv()
            .map_err(|e| BrainBuilderError::Hardware(format!("wgpu map channel closed: {e}")))?
            .map_err(|e| BrainBuilderError::Hardware(format!("wgpu buffer map failed: {e}")))?;

        let data = slice.get_mapped_range();
        let result: Vec<f32> = bytemuck_cast_vec(&data);
        drop(data);
        staging_buf.unmap();
        Ok(result)
    }
}

fn bytemuck_cast_slice(values: &[f32]) -> &[u8] {
    unsafe { std::slice::from_raw_parts(values.as_ptr().cast::<u8>(), std::mem::size_of_val(values)) }
}

fn bytemuck_cast_vec(bytes: &[u8]) -> Vec<f32> {
    let mut out = vec![0f32; bytes.len() / 4];
    unsafe {
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), out.as_mut_ptr().cast::<u8>(), bytes.len());
    }
    out
}

impl Device for WgpuDevice {
    fn name(&self) -> &str {
        "wgpu"
    }

    fn alloc(&self, shape: &[usize], dtype: DataType) -> Tensor {
        let dl_dtype = dtype_to_dlpack(dtype);
        let shape_i64: Vec<i64> = shape.iter().map(|&x| x as i64).collect();
        let tensor_ptr = alloc_managed_tensor(&shape_i64, dl_dtype, cpu_context());
        Arc::new(TensorHandle(tensor_ptr))
    }

    fn exec(&self, kernel: &str, inputs: &[&Tensor]) -> Result<Tensor> {
        let input = inputs
            .first()
            .ok_or_else(|| BrainBuilderError::ConfigError(format!("{kernel}: expected at least 1 input")))?;
        let (shape, values) = crate::interop::dlpack_support::tensor_to_vec_f32(input)?;

        let shader = match kernel {
            "relu" => RELU_SHADER,
            "identity" => IDENTITY_SHADER,
            other => {
                return Err(BrainBuilderError::UnsupportedLanguage(format!(
                    "no wgpu compute shader registered for `{other}`"
                )))
            }
        };
        let result = self.run_elementwise_shader(shader, &values)?;

        let out_shape: Vec<usize> = shape.iter().map(|&d| d as usize).collect();
        let out_tensor = self.alloc(&out_shape, DataType::Float32);
        unsafe {
            let dst = (*out_tensor.0).dl_tensor.data as *mut f32;
            std::ptr::copy_nonoverlapping(result.as_ptr(), dst, result.len());
        }
        Ok(out_tensor)
    }

    fn sync(&self) -> Result<()> {
        self.device.poll(wgpu::Maintain::Wait);
        Ok(())
    }
}

#[cfg(test)]
mod adapter_tests {
    use super::*;

    // Enumeration must never panic and must yield well-formed entries. On a
    // headless CI box the list may be empty; on a real machine it lists the
    // installed GPUs (e.g. an AMD RX 7900 XTX). Either way each entry is sane.
    #[test]
    fn list_adapters_is_well_formed() {
        let adapters = list_adapters();
        for a in &adapters {
            assert!(!a.name.is_empty(), "adapter name should not be empty");
            assert!(!a.backend.is_empty(), "adapter backend should not be empty");
        }
    }

    // A preference that matches nothing must fall back rather than fail to bind
    // a GPU — but only assert that when a GPU actually exists here.
    #[test]
    fn unmatched_preference_falls_back_when_a_gpu_exists() {
        if list_adapters().is_empty() {
            return; // no GPU in this environment; nothing to bind
        }
        let dev = WgpuDevice::with_preferred(Some("no-such-gpu-zzz"));
        assert!(dev.is_ok(), "should fall back to the default adapter, not error");
    }
}
