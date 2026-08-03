use crate::nervous_system::nervous_system::NervousSystem;
use crate::nervous_system::registry::ProviderRegistry;
use crate::nervous_system::types::{SandboxSpec, NervousSystemStatus};
use tauri::State;

pub struct NervousSystemState {
    pub registry: ProviderRegistry,
    pub system: NervousSystem,
}

#[tauri::command]
pub fn nervous_system_status(
    state: State<'_, NervousSystemState>,
) -> Result<NervousSystemStatus, String> {
    Ok(NervousSystemStatus {
        providers: Vec::new(),
        jobs: Vec::new(),
        total_jobs: 0,
        active_jobs: 0,
        failed_jobs_24h: 0,
    })
}

#[tauri::command]
pub async fn nervous_system_providers(
    state: State<'_, NervousSystemState>,
) -> Result<Vec<crate::nervous_system::types::ProviderCapabilities>, String> {
    let caps = futures::future::join_all(
        state.registry.all().iter().map(|p| async move { p.capabilities().await })
    ).await;
    Ok(caps)
}

#[tauri::command]
pub async fn nervous_system_submit(
    state: State<'_, NervousSystemState>,
    provider_id: String,
    spec: SandboxSpec,
) -> Result<crate::nervous_system::types::SandboxJob, String> {
    state.system.submit(&provider_id, spec).await
}

#[tauri::command]
pub async fn nervous_system_start(
    state: State<'_, NervousSystemState>,
    job_id: String,
) -> Result<(), String> {
    state.system.start(&job_id).await
}

#[tauri::command]
pub async fn nervous_system_stop(
    state: State<'_, NervousSystemState>,
    job_id: String,
) -> Result<(), String> {
    state.system.stop(&job_id).await
}

#[tauri::command]
pub async fn nervous_system_list(
    state: State<'_, NervousSystemState>,
) -> Result<Vec<crate::nervous_system::types::SandboxJob>, String> {
    Ok(state.system.list().await)
}
