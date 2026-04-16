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
    let room_name = format!("desktop-{did}-{}", chrono::Utc::now().timestamp());
    let session_id = Uuid::new_v4().to_string();
    let stale_session_id = st.device_sessions.write().await.insert(did, session_id.clone());

    if let Some(old_session_id) = &stale_session_id {
        st.desktop_sessions.write().await.remove(old_session_id);
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
    {
        let mut sessions = st.desktop_sessions.write().await;
        let session = sessions
            .get(&query.session_id)
            .ok_or_else(|| DmsxError::NotFound("desktop session not found".into()))?;
        if session.tenant_id != tid || session.device_id != did {
            return Err(DmsxError::NotFound("desktop session not found".into()));
        }
        sessions.remove(&query.session_id);
    }

    let mut device_sessions = st.device_sessions.write().await;
    if device_sessions.get(&did) == Some(&query.session_id) {
        device_sessions.remove(&did);
    }

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
    command_repo::create_command(&mut *tx, tenant_id, &cmd)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
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
    command_repo::create_command(&mut *tx, tenant_id, &cmd)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(())
}
