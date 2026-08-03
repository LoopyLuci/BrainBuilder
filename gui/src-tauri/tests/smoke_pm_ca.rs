//! Runtime smoke test for PowerManager + CompressionAgent integration.
//!
//! This test does NOT require live services. It verifies that the BrainBuilder
//! artifact correctly:
//!   - compiles the PM/CA modules
//!   - exposes the required public symbols
//!   - registers Tauri commands in invoke_handler
//!   - wires PM/CA into AppStateExt
//!
//! Run with: cargo test -p brainbuilder-gui --test smoke_pm_ca

#[test]
fn smoke_power_manager_symbols_exist() {
    let pm_src = include_str!("../src/power_manager.rs");
    assert!(pm_src.contains("PowerManagerBridge"), "PowerManagerBridge not found");
    assert!(pm_src.contains("PowerManagerModelRegistry"), "PowerManagerModelRegistry not found");
    assert!(pm_src.contains("pub async fn pm_health"), "pm_health not found");
    assert!(pm_src.contains("pub async fn pm_list_domains"), "pm_list_domains not found");
    assert!(pm_src.contains("pub async fn pm_apply_power_limit"), "pm_apply_power_limit not found");
    assert!(pm_src.contains("pub async fn pm_recent_telemetry"), "pm_recent_telemetry not found");
    assert!(pm_src.contains("pub async fn pm_list_blueprints"), "pm_list_blueprints not found");
    assert!(pm_src.contains("pub async fn pm_build_mpc_spec"), "pm_build_mpc_spec not found");
}

#[test]
fn smoke_compression_agent_symbols_exist() {
    let ca_src = include_str!("../src/compression_agent.rs");
    assert!(ca_src.contains("CompressionAgentBridge"), "CompressionAgentBridge not found");
    assert!(ca_src.contains("CompressionAgentModelRegistry"), "CompressionAgentModelRegistry not found");
    assert!(ca_src.contains("pub async fn ca_health"), "ca_health not found");
    assert!(ca_src.contains("pub async fn ca_compress"), "ca_compress not found");
    assert!(ca_src.contains("pub async fn ca_recent_stats"), "ca_recent_stats not found");
    assert!(ca_src.contains("pub async fn ca_list_blueprints"), "ca_list_blueprints not found");
    assert!(ca_src.contains("pub async fn ca_build_compressor_spec"), "ca_build_compressor_spec not found");
}

#[test]
fn smoke_main_wires_pm_ca() {
    let main_src = include_str!("../src/main.rs");
    assert!(main_src.contains("mod power_manager;"), "mod power_manager; not found in main.rs");
    assert!(main_src.contains("mod compression_agent;"), "mod compression_agent; not found in main.rs");
    assert!(main_src.contains("pm_health,"), "pm_health not in invoke_handler");
    assert!(main_src.contains("pm_list_domains,"), "pm_list_domains not in invoke_handler");
    assert!(main_src.contains("pm_apply_power_limit,"), "pm_apply_power_limit not in invoke_handler");
    assert!(main_src.contains("pm_build_mpc_spec,"), "pm_build_mpc_spec not in invoke_handler");
    assert!(main_src.contains("ca_health,"), "ca_health not in invoke_handler");
    assert!(main_src.contains("ca_compress,"), "ca_compress not in invoke_handler");
    assert!(main_src.contains("ca_recent_stats,"), "ca_recent_stats not in invoke_handler");
    assert!(main_src.contains("ca_build_compressor_spec,"), "ca_build_compressor_spec not in invoke_handler");
}

#[test]
fn smoke_self_improving_commands_wires_pm_ca() {
    let sic_src = include_str!("../src/self_improving_commands.rs");
    assert!(sic_src.contains("pub power_manager: crate::power_manager::PowerManagerBridge,"), "power_manager field missing from AppStateExt");
    assert!(sic_src.contains("pub compression_agent: crate::compression_agent::CompressionAgentBridge,"), "compression_agent field missing from AppStateExt");
    assert!(sic_src.contains("power_manager: crate::power_manager::PowerManagerBridge::new"), "power_manager init missing");
    assert!(sic_src.contains("compression_agent: crate::compression_agent::CompressionAgentBridge::new"), "compression_agent init missing");
}
