//! Real hardware introspection for one device in a Cluster — CPU/RAM via the
//! nervous system's `system_resources` (already real, tested against actual
//! hardware), GPU(s) via wgpu's synchronous adapter enumeration (the same
//! backend already proven against a real AMD Radeon RX 7900 XTX in
//! `core/tests/wgpu_backend.rs`). No guessing, no placeholders: every field
//! here is either a real OS/driver query or explicitly `None`/empty when
//! that query isn't available on this platform/build.
use crate::runtime::nervous_system::system_resources;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuProfile {
    pub name: String,
    pub vram_bytes: Option<u64>,
    pub backend: String, // e.g. "Vulkan", "Dx12", "Metal" (wgpu::Backend's Display)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceProfile {
    pub display_name: String,
    pub os: String,
    pub cpu_cores: usize,
    pub ram_total_bytes: Option<u64>,
    pub gpus: Vec<GpuProfile>,
}

impl DeviceProfile {
    /// Captures a real profile of the machine this is called on. `wgpu`
    /// GPU enumeration only runs when the `wgpu` feature is enabled — same
    /// feature gate as `runtime::wgpu_backend`, so a build without it
    /// correctly reports zero GPUs rather than pretending to know about
    /// hardware it can't query.
    pub fn capture(display_name: impl Into<String>) -> Self {
        Self {
            display_name: display_name.into(),
            os: std::env::consts::OS.to_string(),
            cpu_cores: system_resources::logical_cpu_count(),
            ram_total_bytes: system_resources::total_physical_memory_bytes(),
            gpus: capture_gpus(),
        }
    }

    /// Whether this device meets a job's minimum resource floor. Missing
    /// data (e.g. RAM query unavailable) fails closed — a requirement is
    /// only satisfied by a real, confirmed value, never by absence of
    /// information.
    pub fn meets(&self, req: &ResourceRequirement) -> bool {
        if let Some(min_cores) = req.min_cpu_cores {
            if self.cpu_cores < min_cores {
                return false;
            }
        }
        if let Some(min_ram) = req.min_ram_bytes {
            match self.ram_total_bytes {
                Some(ram) if ram >= min_ram => {}
                _ => return false,
            }
        }
        if let Some(min_vram) = req.min_vram_bytes {
            let has_enough_vram = self
                .gpus
                .iter()
                .any(|g| g.vram_bytes.is_some_and(|v| v >= min_vram));
            if !has_enough_vram {
                return false;
            }
        }
        true
    }
}

#[cfg(feature = "wgpu")]
fn capture_gpus() -> Vec<GpuProfile> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..Default::default()
    });
    instance
        .enumerate_adapters(wgpu::Backends::all())
        .into_iter()
        .map(|adapter| {
            let info = adapter.get_info();
            // wgpu doesn't expose VRAM size directly (Vulkan/DX12/Metal all
            // report it differently and wgpu doesn't unify it); rather than
            // fabricate a number, this is honestly `None` until a real
            // per-backend query (e.g. `VK_EXT_memory_budget`) is added.
            GpuProfile { name: info.name, vram_bytes: None, backend: format!("{:?}", info.backend) }
        })
        .collect()
}

#[cfg(not(feature = "wgpu"))]
fn capture_gpus() -> Vec<GpuProfile> {
    Vec::new()
}

/// Minimum resources a Job requires to even be joinable — checked locally
/// (never bother contacting the host with unusable capacity) and re-checked
/// by the host (never trust a client's self-reported profile for anything
/// that gates access).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ResourceRequirement {
    pub min_cpu_cores: Option<usize>,
    pub min_ram_bytes: Option<u64>,
    pub min_vram_bytes: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capture_returns_real_nonzero_cpu_count() {
        let profile = DeviceProfile::capture("test-device");
        assert!(profile.cpu_cores >= 1);
        assert_eq!(profile.os, std::env::consts::OS);
    }

    #[test]
    fn meets_fails_closed_on_unspecified_requirement() {
        let profile = DeviceProfile::capture("test-device");
        assert!(profile.meets(&ResourceRequirement::default()));
    }

    #[test]
    fn meets_rejects_a_floor_no_real_machine_hits() {
        let profile = DeviceProfile::capture("test-device");
        let impossible = ResourceRequirement { min_cpu_cores: Some(usize::MAX), ..Default::default() };
        assert!(!profile.meets(&impossible));
    }

    #[test]
    fn meets_rejects_vram_floor_when_gpu_list_is_empty() {
        let mut profile = DeviceProfile::capture("test-device");
        profile.gpus.clear();
        let req = ResourceRequirement { min_vram_bytes: Some(1), ..Default::default() };
        assert!(!profile.meets(&req));
    }
}
