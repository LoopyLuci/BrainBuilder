use std::collections::VecDeque;
use std::sync::Arc;
use tauri::{command, State};
use tokio::sync::Mutex;
use chrono::Utc;
use crate::AppState;
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BotSettings {
    pub platform: String,
    pub enabled: bool,
    pub credentials: std::collections::HashMap<String, String>,
    pub home_channel: Option<String>,
    pub allowed_users: Vec<String>,
    pub proxy: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BotStatus {
    pub running: bool,
    pub platform: String,
    pub uptime_secs: u64,
    pub last_error: Option<String>,
    pub started_at: Option<i64>,
    pub message_count: u64,
    pub command_count: u64,
    pub error_count: u64,
    pub ping_ms: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BotTelemetry {
    pub ping_ms: Option<u64>,
    pub upload_mbps: f64,
    pub download_mbps: f64,
    pub jitter_ms: Option<f64>,
    pub response_time_ms: Option<u64>,
    pub cpu_percent: f64,
    pub memory_mb: u64,
    pub adapter_load: std::collections::HashMap<String, f64>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct BotEvent {
    pub id: u64,
    pub kind: String,
    pub message: String,
    pub at: i64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StartConfig {
    pub platform: String,
    pub credentials: std::collections::HashMap<String, String>,
    pub home_channel: Option<String>,
    pub allowed_users: Vec<String>,
    pub proxy: Option<String>,
}

pub struct BotServerHandle {
    running: bool,
    platform: String,
    started_at: Option<i64>,
    message_count: u64,
    command_count: u64,
    error_count: u64,
    last_error: Option<String>,
    settings: BotSettings,
    events: VecDeque<BotEvent>,
    telemetry: BotTelemetry,
}

impl Default for BotServerHandle {
    fn default() -> Self {
        Self {
            running: false,
            platform: String::new(),
            started_at: None,
            message_count: 0,
            command_count: 0,
            error_count: 0,
            last_error: None,
            settings: BotSettings {
                platform: String::new(),
                enabled: true,
                credentials: std::collections::HashMap::new(),
                home_channel: None,
                allowed_users: Vec::new(),
                proxy: None,
            },
            events: VecDeque::with_capacity(256),
            telemetry: BotTelemetry {
                ping_ms: None,
                upload_mbps: 0.0,
                download_mbps: 0.0,
                jitter_ms: None,
                response_time_ms: None,
                cpu_percent: 0.0,
                memory_mb: 0,
                adapter_load: std::collections::HashMap::new(),
            },
        }
    }
}

#[command]
pub async fn bot_dashboard_status(state: State<'_, AppState>) -> Result<BotStatus, String> {
    let handle = state.bot.lock().await;
    let uptime = handle.started_at.map(|t| (Utc::now().timestamp() - t).max(0) as u64).unwrap_or(0);
    Ok(BotStatus {
        running: handle.running,
        platform: handle.platform.clone(),
        uptime_secs: uptime,
        last_error: handle.last_error.clone(),
        started_at: handle.started_at,
        message_count: handle.message_count,
        command_count: handle.command_count,
        error_count: handle.error_count,
        ping_ms: handle.telemetry.ping_ms,
    })
}

#[command]
pub async fn bot_dashboard_settings_get(state: State<'_, AppState>) -> Result<BotSettings, String> {
    let handle = state.bot.lock().await;
    Ok(handle.settings.clone())
}

#[command]
pub async fn bot_dashboard_settings_set(settings: BotSettings, state: State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.bot.lock().await;
    handle.settings = settings.clone();
    handle.events.push_front(BotEvent { id: 0, kind: "settings".into(), message: "settings updated".into(), at: Utc::now().timestamp() });
    Ok(())
}

#[command]
pub async fn bot_dashboard_start(config: StartConfig, state: State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.bot.lock().await;
    handle.running = true;
    handle.platform = config.platform.clone();
    handle.settings = BotSettings { platform: config.platform, enabled: true, credentials: config.credentials, home_channel: config.home_channel, allowed_users: config.allowed_users, proxy: config.proxy };
    handle.started_at = Some(Utc::now().timestamp());
    handle.last_error = None;
    handle.events.push_front(BotEvent { id: 0, kind: "lifecycle".into(), message: "started".into(), at: Utc::now().timestamp() });
    Ok(())
}

#[command]
pub async fn bot_dashboard_stop(state: State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.bot.lock().await;
    handle.running = false;
    handle.events.push_front(BotEvent { id: 0, kind: "lifecycle".into(), message: "stopped".into(), at: Utc::now().timestamp() });
    Ok(())
}

#[command]
pub async fn bot_dashboard_restart(config: StartConfig, state: State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.bot.lock().await;
    handle.running = true;
    handle.platform = config.platform.clone();
    handle.settings = BotSettings { platform: config.platform, enabled: true, credentials: config.credentials, home_channel: config.home_channel, allowed_users: config.allowed_users, proxy: config.proxy };
    handle.started_at = Some(Utc::now().timestamp());
    handle.last_error = None;
    handle.message_count = 0;
    handle.command_count = 0;
    handle.error_count = 0;
    handle.events.push_front(BotEvent { id: 0, kind: "lifecycle".into(), message: "restarted".into(), at: Utc::now().timestamp() });
    Ok(())
}

#[command]
pub async fn bot_dashboard_events(state: State<'_, AppState>) -> Result<Vec<BotEvent>, String> {
    let handle = state.bot.lock().await;
    Ok(handle.events.iter().cloned().collect())
}

#[command]
pub async fn bot_dashboard_clear_events(state: State<'_, AppState>) -> Result<(), String> {
    let mut handle = state.bot.lock().await;
    handle.events.clear();
    Ok(())
}

#[command]
pub async fn bot_dashboard_telemetry(state: State<'_, AppState>) -> Result<BotTelemetry, String> {
    let mut handle = state.bot.lock().await;
    let ping = (rand::random::<u64>() % 40) + 5;
    let jitter = (rand::random::<u64>() % 20) as f64 + 1.0;
    let rt = ping + (rand::random::<u64>() % 20);
    let cpu = (rand::random::<u64>() % 60) as f64 + 2.0;
    let mem = (rand::random::<u64>() % 400) + 120;
    let load = handle.telemetry.adapter_load.get(handle.platform.as_str()).copied().unwrap_or(0.0);
    handle.telemetry = BotTelemetry { ping_ms: Some(ping), upload_mbps: load + 0.3, download_mbps: load + 0.7, jitter_ms: Some(jitter), response_time_ms: Some(rt), cpu_percent: cpu, memory_mb: mem, adapter_load: handle.telemetry.adapter_load.clone() };
    Ok(handle.telemetry.clone())
}

#[command]
pub async fn bot_autostart(state: State<'_, AppState>) -> Result<(), String> {
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") { Ok(t) => t, Err(_) => return Ok(()) };
    let mut handle = state.bot.lock().await;
    if handle.running {
        return Ok(());
    }
    let mut credentials = std::collections::HashMap::new();
    credentials.insert("token".into(), token);
    let allowed = std::env::var("TELEGRAM_ALLOWED_USERS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>())
        .unwrap_or_default();
    handle.running = true;
    handle.platform = "telegram".into();
    handle.settings = BotSettings {
        platform: "telegram".into(),
        enabled: true,
        credentials,
        home_channel: std::env::var("TELEGRAM_HOME_CHANNEL").ok(),
        allowed_users: allowed,
        proxy: std::env::var("TELEGRAM_PROXY").ok(),
    };
    handle.started_at = Some(chrono::Utc::now().timestamp());
    handle.last_error = None;
    handle.events.push_front(BotEvent { id: 0, kind: "lifecycle".into(), message: "autostarted".into(), at: chrono::Utc::now().timestamp() });
    Ok(())
}

pub fn try_autostart(handle: &Arc<Mutex<BotServerHandle>>) {
    let rt = match tokio::runtime::Runtime::new() { Ok(r) => r, Err(_) => return };
    let handle = handle.clone();
    rt.spawn(async move {
        let _ = do_autostart(handle).await;
    });
}

async fn do_autostart(handle: Arc<Mutex<BotServerHandle>>) -> Result<(), String> {
    let token = match std::env::var("TELEGRAM_BOT_TOKEN") { Ok(t) => t, Err(_) => return Ok(()) };
    let mut guard = handle.lock().await;
    if guard.running {
        return Ok(());
    }
    let mut credentials = std::collections::HashMap::new();
    credentials.insert("token".into(), token);
    let allowed = std::env::var("TELEGRAM_ALLOWED_USERS")
        .map(|v| v.split(',').map(|s| s.trim().to_string()).collect::<Vec<String>>())
        .unwrap_or_default();
    guard.running = true;
    guard.platform = "telegram".into();
    guard.settings = BotSettings {
        platform: "telegram".into(),
        enabled: true,
        credentials,
        home_channel: std::env::var("TELEGRAM_HOME_CHANNEL").ok(),
        allowed_users: allowed,
        proxy: std::env::var("TELEGRAM_PROXY").ok(),
    };
    guard.started_at = Some(chrono::Utc::now().timestamp());
    guard.last_error = None;
    guard.events.push_front(BotEvent { id: 0, kind: "lifecycle".into(), message: "autostarted".into(), at: chrono::Utc::now().timestamp() });
    Ok(())
}
