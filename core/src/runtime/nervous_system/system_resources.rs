//! Real system resource queries, used to auto-scale sandbox memory ceilings
//! instead of hardcoding a number that's wrong for both a 8GB laptop and a
//! 64GB desktop. Windows: `GlobalMemoryStatusEx` (real Win32 API, not a
//! guess). Other platforms: `None` — callers fall back to a conservative
//! fixed default rather than silently pretending to know the real figure.

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};

    pub fn total_physical_memory_bytes() -> Option<u64> {
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) == 0 {
                return None;
            }
            Some(status.ullTotalPhys)
        }
    }

    pub fn available_physical_memory_bytes() -> Option<u64> {
        unsafe {
            let mut status: MEMORYSTATUSEX = std::mem::zeroed();
            status.dwLength = std::mem::size_of::<MEMORYSTATUSEX>() as u32;
            if GlobalMemoryStatusEx(&mut status) == 0 {
                return None;
            }
            Some(status.ullAvailPhys)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn total_physical_memory_bytes() -> Option<u64> {
        None
    }
    pub fn available_physical_memory_bytes() -> Option<u64> {
        None
    }
}

pub use imp::{available_physical_memory_bytes, total_physical_memory_bytes};

/// Real, portable (works everywhere `std` does) logical core count — used to
/// size worker/thread-pool concurrency for distributed training later.
pub fn logical_cpu_count() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}

/// Resolves a sandbox memory ceiling: an explicit `env_var` override (in
/// MiB) wins if set and parses; otherwise auto-scales as `fraction_of_total`
/// of *real* total system RAM (not a one-size-fits-all constant — a fixed
/// 3GiB ceiling is trivial on a 64GiB desktop and dangerously tight on an
/// 8GiB laptop), clamped to `[floor_bytes, ceiling_bytes]` so it's never
/// absurdly small or large. Falls back to `floor_bytes` if the OS memory
/// query itself isn't available (non-Windows today).
pub fn resolve_memory_limit_bytes(
    env_var: &str,
    fraction_of_total: f64,
    floor_bytes: u64,
    ceiling_bytes: u64,
) -> u64 {
    if let Ok(val) = std::env::var(env_var) {
        if let Ok(mb) = val.trim().parse::<u64>() {
            return (mb * 1024 * 1024).clamp(floor_bytes, ceiling_bytes);
        }
    }
    let auto = total_physical_memory_bytes()
        .map(|total| (total as f64 * fraction_of_total) as u64)
        .unwrap_or(floor_bytes);
    auto.clamp(floor_bytes, ceiling_bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_cpu_count_is_at_least_one() {
        assert!(logical_cpu_count() >= 1);
    }

    #[test]
    fn env_override_wins_and_is_clamped() {
        std::env::set_var("BB_TEST_MEMORY_MB", "999999999"); // absurdly high
        let limit = resolve_memory_limit_bytes("BB_TEST_MEMORY_MB", 0.5, 1024, 2048);
        assert_eq!(limit, 2048, "should clamp an out-of-range override to the ceiling");
        std::env::remove_var("BB_TEST_MEMORY_MB");
    }

    #[test]
    fn auto_scales_from_real_total_memory_when_unset() {
        std::env::remove_var("BB_TEST_MEMORY_MB_2");
        let limit = resolve_memory_limit_bytes("BB_TEST_MEMORY_MB_2", 0.5, 1, u64::MAX);
        // On this test machine (real hardware), total RAM is known to be
        // well above 1 byte, so auto-scaling must have actually kicked in
        // rather than falling back to the floor.
        if total_physical_memory_bytes().is_some() {
            assert!(limit > 1);
        }
    }

    #[test]
    #[cfg(windows)]
    fn total_physical_memory_is_a_real_plausible_value() {
        // Sanity bound, not a hardcoded expectation: any real desktop/laptop
        // has at least 1 GiB and (as of this writing) well under 4 TiB.
        let total = total_physical_memory_bytes().expect("GlobalMemoryStatusEx should succeed");
        assert!(total > 1024 * 1024 * 1024);
        assert!(total < 4u64 * 1024 * 1024 * 1024 * 1024);
    }
}
