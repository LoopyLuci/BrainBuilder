use std::net::SocketAddr;
use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::{mpsc, Mutex};
use crate::adapter::PlatformAdapter;
use crate::env_config::LuciEnv;
use crate::luci_bridge::LuciBridge;
use crate::telegram::TelegramAdapter;
use crate::types::{BotConfig, ConversationId, IncomingMessage, LuciAction, OutgoingMessage};
use crate::voice::VoiceServer;
use axum::{Router, routing::get};
use tokio::net::TcpListener;

pub struct BotServerConfig {
    pub adapters: Vec<BotConfig>,
}

impl Default for BotServerConfig {
    fn default() -> Self {
        Self { adapters: Vec::new() }
    }
}

pub struct BotServer {
    _config: BotServerConfig,
}

impl BotServer {
    pub fn new(config: BotServerConfig) -> Self {
        Self { _config: config }
    }
}

pub struct UnifiedBot {
    luci: Mutex<LuciBridge>,
    _voice: VoiceServer,
    incoming: mpsc::UnboundedSender<IncomingMessage>,
    adapters: Mutex<HashMap<String, Box<dyn PlatformAdapter>>>,
}

impl UnifiedBot {
    pub async fn start(addr: SocketAddr, _configs: Vec<BotConfig>) -> Result<Arc<Self>, Box<dyn std::error::Error>> {
        let (tx, mut rx) = mpsc::unbounded_channel::<IncomingMessage>();
        let bot = Arc::new(Self {
            luci: Mutex::new(LuciBridge::new(ConversationId("default".into()))),
            _voice: VoiceServer::default(),
            incoming: tx.clone(),
            adapters: Mutex::new(HashMap::new()),
        });

        // HTTP health/root endpoint
        let app = Router::new()
            .route("/", get(|| async { "BrainBuilder bot server running" }))
            .route("/health", get(|| async { "ok" }));
        let listener = TcpListener::bind(addr).await?;
        tokio::spawn(async move { axum::serve(listener, app).await.ok(); });

        // Central message pump: routes every adapter message through Luci
        let pump = bot.clone();
        tokio::spawn(async move {
            while let Some(message) = rx.recv().await {
                let mut luci = pump.luci.lock().await;
                let response = luci.handle(message).await;
                drop(luci);
                for action in response.actions {
                    pump.apply_action(action).await;
                }
            }
        });

        // Start configured adapters
        let env = LuciEnv::from_env();
        let mut started_any = false;
        if let Some(token) = env.bot_token {
            let mut config = BotConfig::default();
            config.platform = "telegram".into();
            config.credentials.insert("token".into(), token);
            let platform = config.platform.clone();
            let adapter = TelegramAdapter::new_with_sender(Some(bot.incoming.clone()));
            if let Err(start_err) = adapter.start(config).await {
                tracing::warn!(platform = %platform, error = ?start_err, "telegram adapter start failed");
            } else {
                started_any = true;
                let mut adapters_map = bot.adapters.lock().await;
                adapters_map.insert(platform, Box::new(adapter));
            }
        }
        if !started_any {
            tracing::warn!("no adapters started: set TELEGRAM_BOT_TOKEN in .env to enable telegram");
        }

        Ok(bot)
    }

    pub async fn handle(&self, message: IncomingMessage) {
        let _ = self.incoming.send(message);
    }

    async fn apply_action(&self, action: LuciAction) {
        match action {
            LuciAction::SendMessage { conversation, text } => {
                let adapters = self.adapters.lock().await;
                if let Some(adapter) = adapters.values().next() {
                    let _ = adapter.send(OutgoingMessage { conversation, text, media: None }).await;
                }
            }
            LuciAction::SendMedia { media_type, url, bytes } => {
                let adapters = self.adapters.lock().await;
                if let Some(adapter) = adapters.values().next() {
                    let _ = adapter.send(OutgoingMessage {
                        conversation: ConversationId("default".into()),
                        text: String::new(),
                        media: Some(crate::types::OutgoingMedia { media_type, url, bytes }),
                    }).await;
                }
            }
            LuciAction::StartCall { platform } => {
                let _ = platform;
            }
            LuciAction::EndCall { reason } => {
                let _ = reason;
            }
            LuciAction::SetMood { mood } => {
                let mut luci = self.luci.lock().await;
                luci.set_mood(mood);
            }
            LuciAction::Remember { content, tags } => {
                let mut luci = self.luci.lock().await;
                luci.remember_with_tags(content, tags);
            }
            LuciAction::Plan { title, description, steps } => {
                let mut luci = self.luci.lock().await;
                let _ = luci.create_plan(title, description, steps);
            }
        }
    }
}
