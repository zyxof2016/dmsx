use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::auth::AuthConfig;
use crate::command_jetstream::CommandJetStream;

#[derive(Clone, Serialize, Deserialize)]
pub struct DesktopSession {
    pub tenant_id: Uuid,
    pub device_id: Uuid,
}

#[derive(Clone)]
pub struct AppState {
    pub db: PgPool,
    pub redis_url: Option<String>,
    pub command_jetstream: Option<Arc<CommandJetStream>>,
    pub upload_token_hmac_secret: Option<String>,
    pub enroll_token_hmac_secret: Option<String>,
    pub livekit_url: String,
    pub livekit_api_key: String,
    pub livekit_api_secret: String,
    pub desktop_sessions: Arc<RwLock<HashMap<String, DesktopSession>>>,
    pub device_sessions: Arc<RwLock<HashMap<Uuid, String>>>,
    pub auth: AuthConfig,
}
