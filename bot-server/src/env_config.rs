use std::path::Path;

#[derive(Debug, Clone)]
pub struct LuciEnv {
    pub bot_token: Option<String>,
    pub allowed_users: Vec<String>,
    pub home_channel: Option<String>,
    pub home_channel_name: Option<String>,
    pub bot_server_host: String,
    pub bot_server_port: u16,
    pub telegram_proxy: Option<String>,
}

impl Default for LuciEnv {
    fn default() -> Self {
        Self {
            bot_token: std::env::var("TELEGRAM_BOT_TOKEN").ok(),
            allowed_users: std::env::var("TELEGRAM_ALLOWED_USERS")
                .map(|v| v.split(',').map(|s| s.trim().to_string()).collect())
                .unwrap_or_default(),
            home_channel: std::env::var("TELEGRAM_HOME_CHANNEL").ok(),
            home_channel_name: std::env::var("TELEGRAM_HOME_CHANNEL_NAME").ok(),
            bot_server_host: std::env::var("BOT_SERVER_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
            bot_server_port: std::env::var("BOT_SERVER_PORT").ok().and_then(|v| v.parse().ok()).unwrap_or(7780),
            telegram_proxy: std::env::var("TELEGRAM_PROXY").ok(),
        }
    }
}

impl LuciEnv {
    pub fn from_env() -> Self {
        let candidates = [
            std::env::current_dir().ok().and_then(|p| if p.join(".env").exists() { Some(p.join(".env")) } else { None }),
            std::env::var_os("CARGO_MANIFEST_DIR").map(|d| Path::new(&d).join(".env")),
            Some(Path::new("/z/Projects/BrainBuilder").join(".env")),
        ];

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() {
                let _ = dotenvy::dotenv();
                break;
            }
        }
        Self::default()
    }
}
