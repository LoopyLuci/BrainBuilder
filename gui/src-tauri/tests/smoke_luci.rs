//! Smoke tests for Luci assistant integration.
//!
//! Verifies:
//! - LuciStore compiles and exposes persistence API
//! - Luci facade compiles and exposes chat/plan/reflect/memory API
//! - Luci commands are wired into self_improving_commands
//! - Luci is registered in AppStateExt

#[test]
fn luci_store_symbols_exist() {
    let src = include_str!("../src/luci_store.rs");
    assert!(src.contains("pub struct LuciStore"), "LuciStore not found");
    assert!(src.contains("pub async fn new"), "LuciStore::new not found");
    assert!(src.contains("pub async fn remember"), "remember not found");
    assert!(src.contains("pub async fn recall"), "recall not found");
    assert!(src.contains("pub async fn forget"), "forget not found");
    assert!(src.contains("pub async fn save_plan"), "save_plan not found");
    assert!(src.contains("pub async fn list_plans"), "list_plans not found");
    assert!(src.contains("pub async fn save_reflection"), "save_reflection not found");
    assert!(src.contains("pub async fn recent_reflections"), "recent_reflections not found");
    assert!(src.contains("pub async fn set_preference"), "set_preference not found");
    assert!(src.contains("pub async fn get_preference"), "get_preference not found");
    assert!(src.contains("pub async fn register_tool"), "register_tool not found");
    assert!(src.contains("pub async fn list_tools"), "list_tools not found");
    assert!(src.contains("pub async fn audit"), "audit not found");
    assert!(src.contains("pub async fn recent_audit"), "recent_audit not found");
}

#[test]
fn luci_facade_symbols_exist() {
    let src = include_str!("../src/luci.rs");
    assert!(src.contains("pub struct Luci"), "Luci struct not found");
    assert!(src.contains("pub async fn new"), "Luci::new not found");
    assert!(src.contains("pub async fn greet"), "greet not found");
    assert!(src.contains("pub async fn chat"), "chat not found");
    assert!(src.contains("pub async fn propose_plan"), "propose_plan not found");
    assert!(src.contains("pub async fn list_plans"), "list_plans not found");
    assert!(src.contains("pub async fn update_plan_status"), "update_plan_status not found");
    assert!(src.contains("pub async fn reflect"), "reflect not found");
    assert!(src.contains("pub async fn recent_reflections"), "recent_reflections not found");
    assert!(src.contains("pub async fn remember_fact"), "remember_fact not found");
    assert!(src.contains("pub async fn recall_memories"), "recall_memories not found");
    assert!(src.contains("pub async fn forget_memory"), "forget_memory not found");
    assert!(src.contains("pub async fn audit"), "audit not found");
    assert!(src.contains("pub async fn recent_audit"), "recent_audit not found");
    assert!(src.contains("pub async fn register_tool"), "register_tool not found");
    assert!(src.contains("pub async fn list_tools"), "list_tools not found");
    assert!(src.contains("pub async fn improve"), "improve not found");
    assert!(src.contains("pub async fn status"), "status not found");
}

#[test]
fn luci_commands_wired() {
    let src = include_str!("../src/self_improving_commands.rs");
    assert!(src.contains("pub async fn luci_greet"), "luci_greet not found");
    assert!(src.contains("pub async fn luci_chat"), "luci_chat not found");
    assert!(src.contains("pub async fn luci_propose_plan"), "luci_propose_plan not found");
    assert!(src.contains("pub async fn luci_list_plans"), "luci_list_plans not found");
    assert!(src.contains("pub async fn luci_update_plan_status"), "luci_update_plan_status not found");
    assert!(src.contains("pub async fn luci_reflect"), "luci_reflect not found");
    assert!(src.contains("pub async fn luci_recent_reflections"), "luci_recent_reflections not found");
    assert!(src.contains("pub async fn luci_set_preference"), "luci_set_preference not found");
    assert!(src.contains("pub async fn luci_get_preference"), "luci_get_preference not found");
    assert!(src.contains("pub async fn luci_remember_fact"), "luci_remember_fact not found");
    assert!(src.contains("pub async fn luci_recall_memories"), "luci_recall_memories not found");
    assert!(src.contains("pub async fn luci_forget_memory"), "luci_forget_memory not found");
    assert!(src.contains("pub async fn luci_audit"), "luci_audit not found");
    assert!(src.contains("pub async fn luci_recent_audit"), "luci_recent_audit not found");
    assert!(src.contains("pub async fn luci_register_tool"), "luci_register_tool not found");
    assert!(src.contains("pub async fn luci_list_tools"), "luci_list_tools not found");
    assert!(src.contains("pub async fn luci_improve"), "luci_improve not found");
    assert!(src.contains("pub async fn luci_status"), "luci_status not found");
    assert!(src.contains("pub async fn luci_register_skill"), "luci_register_skill not found");
    assert!(src.contains("pub async fn luci_list_skills"), "luci_list_skills not found");
    assert!(src.contains("pub async fn luci_observe_and_learn"), "luci_observe_and_learn not found");
    assert!(src.contains("pub async fn luci_imitate_skill"), "luci_imitate_skill not found");
    assert!(src.contains("pub async fn luci_decompose_task"), "luci_decompose_task not found");
    assert!(src.contains("pub async fn luci_register_model"), "luci_register_model not found");
    assert!(src.contains("pub async fn luci_list_models"), "luci_list_models not found");
    assert!(src.contains("pub async fn luci_register_dataset"), "luci_register_dataset not found");
    assert!(src.contains("pub async fn luci_list_datasets"), "luci_list_datasets not found");
    assert!(src.contains("pub async fn luci_start_training"), "luci_start_training not found");
    assert!(src.contains("pub async fn luci_list_training_jobs"), "luci_list_training_jobs not found");
}

#[test]
fn luci_registered_in_app_state() {
    let src = include_str!("../src/self_improving_commands.rs");
    assert!(src.contains("pub luci: crate::luci::Luci,"), "luci field missing from AppStateExt");
    assert!(src.contains("Luci::new"), "Luci::new call missing");
}

#[test]
fn nervous_system_symbols_exist() {
    let src = include_str!("../src/nervous_system/mod.rs");
    assert!(src.contains("pub mod types"), "nervous_system types module missing");
    assert!(src.contains("pub mod provider"), "nervous_system provider module missing");
    assert!(src.contains("pub mod nervous_system"), "nervous_system core module missing");
    assert!(src.contains("pub mod providers"), "nervous_system providers module missing");
    assert!(src.contains("pub mod registry"), "nervous_system registry module missing");
    assert!(src.contains("pub mod commands"), "nervous_system commands module missing");
}

#[test]
fn nervous_system_commands_wired() {
    let src = include_str!("../src/nervous_system/commands.rs");
    assert!(src.contains("pub async fn nervous_system_providers"), "nervous_system_providers not found");
    assert!(src.contains("pub async fn nervous_system_submit"), "nervous_system_submit not found");
    assert!(src.contains("pub async fn nervous_system_start"), "nervous_system_start not found");
    assert!(src.contains("pub async fn nervous_system_stop"), "nervous_system_stop not found");
    assert!(src.contains("pub async fn nervous_system_list"), "nervous_system_list not found");
}

#[test]
fn tool_executor_symbols_exist() {
    let src = include_str!("../src/tool_executor/mod.rs");
    assert!(src.contains("pub mod types"), "tool_executor types module missing");
    assert!(src.contains("pub mod executor"), "tool_executor executor module missing");
    assert!(src.contains("pub mod registry"), "tool_executor registry module missing");
    assert!(src.contains("pub mod commands"), "tool_executor commands module missing");
    assert!(src.contains("pub mod world"), "tool_executor world module missing");
    assert!(src.contains("pub mod model_ops"), "tool_executor model_ops module missing");
    assert!(src.contains("pub mod learning"), "tool_executor learning module missing");
    assert!(src.contains("pub mod safety"), "tool_executor safety module missing");
    assert!(src.contains("pub mod vector_memory"), "tool_executor vector_memory module missing");
    assert!(src.contains("pub mod knowledge_graph"), "tool_executor knowledge_graph module missing");
}
