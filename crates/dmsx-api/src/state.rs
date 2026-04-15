use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

#[derive(Clone)]
pub struct DesktopRelay {
    pub frames_tx: broadcast::Sender<Vec<u8>>,
    pub input_tx: broadcast::Sender<Vec<u8>>,
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub desktop_sessions: Arc<RwLock<HashMap<String, DesktopRelay>>>,
}
