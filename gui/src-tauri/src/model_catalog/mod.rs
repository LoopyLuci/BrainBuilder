pub mod registry;
pub mod domain_01;
pub mod domain_02;
pub mod domain_03;
pub mod domain_04;
pub mod domain_05;
pub mod domain_06;
pub mod domain_07;
pub mod domain_08;
pub mod domain_09;
pub mod domain_10;
pub mod domain_11;
pub mod domain_12;
pub mod domain_13;
pub mod domain_14;
pub mod domain_15;
pub mod domain_16;
pub mod domain_17;
pub mod domain_18;
pub mod domain_19;
pub mod domain_20;
pub mod domain_21;

pub fn register_all(registry: &mut ModelRegistry) {
    domain_01::register(registry);
    domain_02::register(registry);
    domain_03::register(registry);
    domain_04::register(registry);
    domain_05::register(registry);
    domain_06::register(registry);
    domain_07::register(registry);
    domain_08::register(registry);
    domain_09::register(registry);
    domain_10::register(registry);
    domain_11::register(registry);
    domain_12::register(registry);
    domain_13::register(registry);
    domain_14::register(registry);
    domain_15::register(registry);
    domain_16::register(registry);
    domain_17::register(registry);
    domain_18::register(registry);
    domain_19::register(registry);
    domain_20::register(registry);
    domain_21::register(registry);
}

use std::collections::HashMap;
use serde_json::Value;
use crate::model_catalog::registry::ModelRegistry;
use crate::self_improving_commands::AppStateExt;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn catalog_model_run(
    state: tauri::State<'_, std::sync::Arc<Mutex<AppStateExt>>>,
    model_id: String,
    params: HashMap<String, Value>,
) -> Result<Value, String> {
    let ext = state.lock().await;
    let registry = ext.catalog_registry.read().await;
    registry.execute(&model_id, params).await
}
