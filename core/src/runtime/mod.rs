pub mod trainer;
pub mod standard_trainer;
pub mod scheduler;
pub mod device;
pub mod arrow_bridge;
pub mod cpu_backend;
pub mod nervous_system;
pub mod ops;
#[cfg(feature = "gpu")]
pub mod cuda_backend;
// Gated on `wgpu` directly (not `gpu`) so it's buildable/testable
// independent of `cudarc`, whose build script requires a real CUDA toolkit
// (`nvcc`) just to configure — unrelated to whether wgpu itself works.
#[cfg(feature = "wgpu")]
pub mod wgpu_backend;
pub mod distributed;
