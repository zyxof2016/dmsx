use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Clone)]
pub struct DesktopSession {
    pub tenant_id: Uuid,
    pub device_id: Uuid,
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub desktop_sessions: Arc<RwLock<HashMap<String, DesktopSession>>>,
    pub device_sessions: Arc<RwLock<HashMap<Uuid, String>>>,
}
