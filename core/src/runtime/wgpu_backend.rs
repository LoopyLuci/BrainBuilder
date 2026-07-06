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

impl WgpuDevice {
    /// Picks whatever real adapter the OS/driver gives us first (the point
    /// of wgpu — no vendor-specific setup) and blocks synchronously on its
    /// async setup via `pollster`, since `Device::alloc`/`exec` are sync.
    pub fn new() -> Result<Self> {
        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });
        let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: wgpu::PowerPreference::HighPerformance,
            ..Default::default()
        }))
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
