use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use dmsx_core::DmsxError;
use livekit_api::access_token::{AccessToken, VideoGrants};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::desktop_helpers::{
    build_start_desktop_command, build_stop_desktop_command, livekit_enabled,
};
use crate::error::map_db_error;
use crate::repo::commands as command_repo;
use crate::state::{AppState, DesktopSession};
use redis::AsyncCommands;

fn desktop_session_key(session_id: &str) -> String {
    format!("dmsx:desktop_sessions:{session_id}")
}

fn device_session_key(device_id: Uuid) -> String {
    format!("dmsx:device_sessions:{device_id}")
}

async fn redis_set_session(
    st: &AppState,
    tenant_id: Uuid,
    device_id: Uuid,
    session_id: &str,
) {
    let Some(redis_url) = st.redis_url.as_deref() else {
        return;
    };

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "redis client open failed");
            return;
        }
    };

    let ds = DesktopSession {
        tenant_id,
        device_id,
    };
    let ds_json = match serde_json::to_string(&ds) {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!(error = %e, "redis desktop session json serialize failed");
            return;
        }
    };

    // If Redis isn't reachable we intentionally ignore errors; PG + in-memory behavior stays intact.
    if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
        let _: redis::RedisResult<()> = conn
            .set(desktop_session_key(session_id), ds_json)
            .await;
        let _: redis::RedisResult<()> = conn
            .set(device_session_key(device_id), session_id)
            .await;
    }
}

async fn redis_get_session(st: &AppState, session_id: &str) -> Option<DesktopSession> {
    let Some(redis_url) = st.redis_url.as_deref() else {
        return None;
    };

    let client = redis::Client::open(redis_url).ok()?;
    let mut conn = client.get_multiplexed_async_connection().await.ok()?;

    let key = desktop_session_key(session_id);
    let json: Option<String> = conn.get(key).await.ok()?;
    let ds: DesktopSession = serde_json::from_str(&json?).ok()?;
    Some(ds)
}

async fn redis_delete_session(st: &AppState, session_id: &str, device_id: Uuid) {
    let Some(redis_url) = st.redis_url.as_deref() else {
        return;
    };

    let client = match redis::Client::open(redis_url) {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "redis client open failed");
            return;
        }
    };

    if let Ok(mut conn) = client.get_multiplexed_async_connection().await {
        let _: redis::RedisResult<usize> = conn.del(desktop_session_key(session_id)).await;
        let _: redis::RedisResult<usize> = conn.del(device_session_key(device_id)).await;
    }
}

// ---------------------------------------------------------------------------
// DTOs
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct DesktopSessionResponse {
    pub room: String,
    pub token: String,
    pub livekit_url: String,
    pub session_id: String,
}

#[derive(Serialize)]
pub struct LivekitConfigResponse {
    pub enabled: bool,
    pub url: String,
}

#[derive(Deserialize)]
pub struct SessionCreateReq {
    #[serde(default = "default_resolution_w")]
    pub width: u32,
    #[serde(default = "default_resolution_h")]
    pub height: u32,
}

fn default_resolution_w() -> u32 {
    1920
}
fn default_resolution_h() -> u32 {
    1080
}

#[derive(Deserialize)]
pub struct SessionDeleteQuery {
    pub session_id: String,
}

// ---------------------------------------------------------------------------
// GET /v1/config/livekit
// ---------------------------------------------------------------------------

pub async fn livekit_config(
    State(st): State<AppState>,
    Extension(_ctx): Extension<AuthContext>,
) -> impl IntoResponse {
    Json(LivekitConfigResponse {
        enabled: livekit_enabled(&st.livekit_url, &st.livekit_api_key),
        url: st.livekit_url.clone(),
    })
}

// ---------------------------------------------------------------------------
// POST /v1/tenants/{tid}/devices/{did}/desktop/session
// ---------------------------------------------------------------------------

pub async fn session_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(req): Json<SessionCreateReq>,
) -> Result<impl IntoResponse, DmsxError> {
    let session_id = Uuid::new_v4().to_string();
    let room_name = format!("desktop-{did}-{session_id}");
    let stale_session_id = st.device_sessions.write().await.insert(did, session_id.clone());

    if let Some(old_session_id) = &stale_session_id {
        st.desktop_sessions.write().await.remove(old_session_id);
        redis_delete_session(&st, old_session_id, did).await;
        enqueue_stop_command(&st, &ctx, tid, did, old_session_id.clone(), Some(5)).await?;
    }

    let viewer_token = generate_token(
        &st.livekit_api_key,
        &st.livekit_api_secret,
        &room_name,
        &format!("admin-{session_id}"),
        false,
        true,
        true,
    )
    .map_err(|e| DmsxError::Internal(format!("token generation failed: {e}")))?;

    let agent_token = generate_token(
        &st.livekit_api_key,
        &st.livekit_api_secret,
        &room_name,
        &format!("agent-{did}"),
        true,
        true,
        false,
    )
    .map_err(|e| DmsxError::Internal(format!("token generation failed: {e}")))?;

    st.desktop_sessions
        .write()
        .await
        .insert(
            session_id.clone(),
            DesktopSession {
                tenant_id: tid,
                device_id: did,
            },
        );

    redis_set_session(&st, tid, did, &session_id).await;

    enqueue_start_command(
        &st,
        &ctx,
        tid,
        did,
        &session_id,
        &room_name,
        &agent_token,
        req.width,
        req.height,
    )
    .await?;

    Ok((
        StatusCode::CREATED,
        Json(DesktopSessionResponse {
            room: room_name,
            token: viewer_token,
            livekit_url: st.livekit_url.clone(),
            session_id,
        }),
    ))
}

// ---------------------------------------------------------------------------
// DELETE /v1/tenants/{tid}/devices/{did}/desktop/session
// ---------------------------------------------------------------------------

pub async fn session_delete(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Query(query): Query<SessionDeleteQuery>,
) -> Result<impl IntoResponse, DmsxError> {
    let session = {
        let mut sessions = st.desktop_sessions.write().await;
        if let Some(s) = sessions.get(&query.session_id).cloned() {
            Some(s)
        } else {
            // Multi-instance / restart recovery: fall back to Redis mapping.
            let s_opt = redis_get_session(&st, &query.session_id).await;
            if let Some(ds) = s_opt.clone() {
                sessions.insert(query.session_id.clone(), ds);
            }
            s_opt
        }
    }
    .ok_or_else(|| DmsxError::NotFound("desktop session not found".into()))?;

    // Tenant/device mismatch should not leak which sessions exist.
    if session.tenant_id != tid || session.device_id != did {
        return Err(DmsxError::NotFound("desktop session not found".into()));
    }

    {
        let mut sessions = st.desktop_sessions.write().await;
        sessions.remove(&query.session_id);
    }

    let mut device_sessions = st.device_sessions.write().await;
    if device_sessions.get(&did) == Some(&query.session_id) {
        device_sessions.remove(&did);
    }

    redis_delete_session(&st, &query.session_id, did).await;

    enqueue_stop_command(&st, &ctx, tid, did, query.session_id, Some(10)).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// LiveKit Token Generation
// ---------------------------------------------------------------------------

fn generate_token(
    api_key: &str,
    api_secret: &str,
    room: &str,
    identity: &str,
    can_publish: bool,
    can_subscribe: bool,
    can_publish_data: bool,
) -> Result<String, String> {
    let grants = VideoGrants {
        room_join: true,
        room: room.to_string(),
        can_publish,
        can_subscribe,
        can_publish_data,
        ..Default::default()
    };

    let token = AccessToken::with_api_key(api_key, api_secret)
        .with_identity(identity)
        .with_grants(grants)
        .to_jwt()
        .map_err(|e| e.to_string())?;

    Ok(token)
}

async fn enqueue_start_command(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
    device_id: Uuid,
    session_id: &str,
    room_name: &str,
    agent_token: &str,
    width: u32,
    height: u32,
) -> Result<(), DmsxError> {
    let cmd = build_start_desktop_command(
        device_id,
        session_id,
        room_name,
        agent_token,
        &st.livekit_url,
        width,
        height,
    );

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::create_command(&mut *tx, tenant_id, &cmd)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    if let Some(js) = &st.command_jetstream {
        js.publish_command_created(&command);
    }
    Ok(())
}

async fn enqueue_stop_command(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
    device_id: Uuid,
    session_id: String,
    priority: Option<i16>,
) -> Result<(), DmsxError> {
    let cmd = build_stop_desktop_command(device_id, &session_id, priority);

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::create_command(&mut *tx, tenant_id, &cmd)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    if let Some(js) = &st.command_jetstream {
        js.publish_command_created(&command);
    }
    Ok(())
}
