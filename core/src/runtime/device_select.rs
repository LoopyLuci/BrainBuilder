//! Resolving the compute device for native (`rust`-language) component ops from
//! a user's GPU preference.
//!
//! Scope, stated honestly: this selects the device that runs BrainBuilder's
//! native `rust` kernels (the `Device::exec` path in `ExecutionPlan::forward`).
//! Python/torch components carry their own device story through the
//! `PythonBridge`; this is the one device-selectable compute path in the Rust
//! runtime, so it's where a "use this GPU" preference actually takes effect.
//!
//! It never fails: with the `wgpu` feature and a matching adapter it returns a
//! GPU device, and in every other case (feature off, no match, bind failure)
//! it falls back to the always-available CPU device.
use crate::runtime::cpu_backend::CpuDevice;
use crate::runtime::device::Device;
use std::sync::Arc;

/// Resolve a compute device from a preferred GPU name (case-insensitive
/// substring, e.g. `"7900 XTX"`). Returns the CPU device when `preferred` is
/// `None`/empty, no adapter matches, the GPU fails to bind, or the crate was
/// built without the `wgpu` feature.
pub fn resolve_device(preferred: Option<&str>) -> Arc<dyn Device> {
    #[cfg(feature = "wgpu")]
    {
        if let Some(name) = preferred {
            if !name.trim().is_empty() {
                match crate::runtime::wgpu_backend::WgpuDevice::with_preferred(Some(name)) {
                    Ok(dev) => {
                        log::info!("native ops running on GPU `{}`", dev.adapter_name());
                        return Arc::new(dev);
                    }
                    Err(e) => {
                        log::warn!("preferred GPU `{name}` unavailable ({e}); using CPU for native ops");
                    }
                }
            }
        }
    }
    #[cfg(not(feature = "wgpu"))]
    let _ = preferred; // no GPU backend compiled in; preference is moot

    Arc::new(CpuDevice)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn none_preference_resolves_to_the_cpu_device() {
        let dev = resolve_device(None);
        assert_eq!(dev.name(), CpuDevice.name());
    }

    #[test]
    fn empty_preference_resolves_to_the_cpu_device() {
        let dev = resolve_device(Some("   "));
        assert_eq!(dev.name(), CpuDevice.name());
    }
}
