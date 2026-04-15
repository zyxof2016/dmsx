use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, State, WebSocketUpgrade,
    },
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use futures_util::{SinkExt, StreamExt};
use livekit_api::access_token::{AccessToken, VideoGrants};
use serde::{Deserialize, Serialize};
use serde_json::json;
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::db;
use crate::dto::CreateCommandReq;
use crate::state::{AppState, DesktopRelay};

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

// ---------------------------------------------------------------------------
// GET /v1/config/livekit
// ---------------------------------------------------------------------------

pub async fn livekit_config(State(st): State<AppState>) -> impl IntoResponse {
    Json(LivekitConfigResponse {
        enabled: !st.livekit_api_key.is_empty(),
        url: st.livekit_url.clone(),
    })
}

// ---------------------------------------------------------------------------
// POST /v1/tenants/{tid}/devices/{did}/desktop/session
// ---------------------------------------------------------------------------

pub async fn session_create(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(req): Json<SessionCreateReq>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    let room_name = format!("desktop-{did}-{}", chrono::Utc::now().timestamp());
    let session_id = Uuid::new_v4().to_string();

    let viewer_token = generate_token(
        &st.livekit_api_key,
        &st.livekit_api_secret,
        &room_name,
        &format!("admin-{session_id}"),
        false,
        true,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"detail": format!("token generation failed: {e}")})),
        )
    })?;

    let agent_token = generate_token(
        &st.livekit_api_key,
        &st.livekit_api_secret,
        &room_name,
        &format!("agent-{did}"),
        true,
        false,
    )
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"detail": format!("token generation failed: {e}")})),
        )
    })?;

    let relay = DesktopRelay {
        frames_tx: broadcast::channel(8).0,
        input_tx: broadcast::channel(64).0,
    };
    st.desktop_sessions
        .write()
        .await
        .insert(did.to_string(), relay);

    let cmd = CreateCommandReq {
        target_device_id: did,
        payload: json!({
            "action": "start_desktop",
            "params": {
                "room": room_name,
                "token": agent_token,
                "livekit_url": st.livekit_url,
                "session_id": session_id,
                "width": req.width,
                "height": req.height,
                "api_ws_url": format!("/v1/tenants/{tid}/devices/{did}/desktop/ws/agent"),
            }
        }),
        priority: Some(10),
        ttl_seconds: Some(120),
        idempotency_key: Some(format!("desktop-{session_id}")),
    };

    let _ = db::create_command(&st.db, tid, &cmd).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"detail": format!("command creation failed: {e}")})),
        )
    })?;

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
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> Result<impl IntoResponse, (StatusCode, Json<serde_json::Value>)> {
    st.desktop_sessions.write().await.remove(&did.to_string());

    let cmd = CreateCommandReq {
        target_device_id: did,
        payload: json!({
            "action": "stop_desktop",
            "params": {}
        }),
        priority: Some(10),
        ttl_seconds: Some(60),
        idempotency_key: None,
    };

    let _ = db::create_command(&st.db, tid, &cmd).await.map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"detail": format!("command creation failed: {e}")})),
        )
    })?;

    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// WebSocket: Viewer (browser admin)
// ---------------------------------------------------------------------------

pub async fn ws_viewer(
    ws: WebSocketUpgrade,
    State(st): State<AppState>,
    Path((_tid, did)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_viewer_ws(socket, st, did.to_string()))
}

async fn handle_viewer_ws(socket: WebSocket, st: AppState, device_id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let relay = {
        let sessions = st.desktop_sessions.read().await;
        sessions.get(&device_id).cloned()
    };

    let Some(relay) = relay else {
        let _ = ws_tx.send(Message::Text(
            json!({"error": "no active desktop session"}).to_string().into(),
        )).await;
        return;
    };

    let mut frames_rx = relay.frames_tx.subscribe();
    let input_tx = relay.input_tx.clone();

    let send_task = tokio::spawn(async move {
        while let Ok(frame) = frames_rx.recv().await {
            if ws_tx.send(Message::Binary(frame.into())).await.is_err() {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Text(text) => {
                    let _ = input_tx.send(text.as_bytes().to_vec());
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }
}

// ---------------------------------------------------------------------------
// WebSocket: Agent (device agent streams frames + receives input)
// ---------------------------------------------------------------------------

pub async fn ws_agent(
    ws: WebSocketUpgrade,
    State(st): State<AppState>,
    Path((_tid, did)): Path<(Uuid, Uuid)>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_agent_ws(socket, st, did.to_string()))
}

async fn handle_agent_ws(socket: WebSocket, st: AppState, device_id: String) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    let relay = {
        let sessions = st.desktop_sessions.read().await;
        sessions.get(&device_id).cloned()
    };

    let Some(relay) = relay else {
        let _ = ws_tx.send(Message::Text(
            json!({"error": "no active desktop session"}).to_string().into(),
        )).await;
        return;
    };

    let frames_tx = relay.frames_tx.clone();
    let mut input_rx = relay.input_tx.subscribe();

    let send_task = tokio::spawn(async move {
        while let Ok(input) = input_rx.recv().await {
            if ws_tx
                .send(Message::Text(String::from_utf8_lossy(&input).into_owned().into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_rx.next().await {
            match msg {
                Message::Binary(data) => {
                    let _ = frames_tx.send(data.to_vec());
                }
                Message::Close(_) => break,
                _ => {}
            }
        }
    });

    tokio::select! {
        _ = send_task => {},
        _ = recv_task => {},
    }

    st.desktop_sessions.write().await.remove(&device_id);
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
) -> Result<String, String> {
    let grants = VideoGrants {
        room_join: true,
        room: room.to_string(),
        can_publish,
        can_subscribe,
        ..Default::default()
    };

    let token = AccessToken::with_api_key(api_key, api_secret)
        .with_identity(identity)
        .with_grants(grants)
        .to_jwt()
        .map_err(|e| e.to_string())?;

    Ok(token)
}
