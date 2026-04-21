use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use std::time::{Duration, Instant};

use tokio::sync::oneshot;
use tracing::{debug, error, info, warn};

use super::capture::{primary_capture_size, spawn_capture_loop};
use super::input::{apply_input_event, InputState};

pub struct DesktopSession {
    pub session_id: String,
    stop_flag: Arc<AtomicBool>,
    handle: tokio::task::JoinHandle<()>,
}

impl DesktopSession {
    pub async fn stop(self) {
        self.stop_flag.store(true, Ordering::Relaxed);
        let _ = self.handle.await;
    }
}

pub async fn start_desktop_session(
    params: &serde_json::Value,
) -> Result<DesktopSession, String> {
    let livekit_url = params
        .get("livekit_url")
        .and_then(|v| v.as_str())
        .ok_or("missing livekit_url in params")?
        .to_string();
    let token = params
        .get("token")
        .and_then(|v| v.as_str())
        .ok_or("missing token in params")?
        .to_string();
    let room = params
        .get("room")
        .and_then(|v| v.as_str())
        .ok_or("missing room in params")?
        .to_string();
    let session_id = params
        .get("session_id")
        .and_then(|v| v.as_str())
        .ok_or("missing session_id in params")?
        .to_string();

    info!(session_id = %session_id, room = %room, livekit_url = %livekit_url, "starting desktop session");

    let stop_flag = Arc::new(AtomicBool::new(false));
    let stop_clone = stop_flag.clone();
    let livekit_url_owned = livekit_url.clone();
    let token_owned = token.clone();
    let room_owned = room.clone();
    let session_id_owned = session_id.clone();
    let (ready_tx, ready_rx) = oneshot::channel();

    let handle = tokio::spawn(async move {
        if let Err(e) = desktop_stream_loop(
            &livekit_url_owned,
            &token_owned,
            &room_owned,
            &session_id_owned,
            stop_clone,
            Some(ready_tx),
        )
        .await
        {
            error!(session_id = %session_id_owned, "desktop session ended with error: {e}");
        }
        info!(session_id = %session_id_owned, "desktop session task exited");
    });

    match ready_rx.await {
        Ok(Ok(())) => {}
        Ok(Err(e)) => {
            stop_flag.store(true, Ordering::Relaxed);
            let _ = handle.await;
            return Err(e);
        }
        Err(_) => {
            stop_flag.store(true, Ordering::Relaxed);
            let _ = handle.await;
            return Err("desktop session task exited before reporting readiness".into());
        }
    }

    Ok(DesktopSession {
        session_id,
        stop_flag,
        handle,
    })
}

async fn desktop_stream_loop(
    livekit_url: &str,
    token: &str,
    room_name: &str,
    session_id: &str,
    stop_flag: Arc<AtomicBool>,
    ready_tx: Option<oneshot::Sender<Result<(), String>>>,
) -> Result<(), String> {
    use libwebrtc::prelude::{RtcVideoSource, VideoResolution};
    use livekit::options::TrackPublishOptions;
    use livekit::prelude::{LocalTrack, LocalVideoTrack, Room, RoomEvent, RoomOptions, TrackSource};

    let (capture_width, capture_height) = primary_capture_size().await?;

    let (room, mut room_events) = match Room::connect(livekit_url, token, RoomOptions::default()).await {
        Ok(v) => v,
        Err(e) => {
            if let Some(tx) = ready_tx {
                let _ = tx.send(Err(format!("livekit connect failed: {e}")));
            }
            return Err(format!("livekit connect failed: {e}"));
        }
    };

    let video_source = libwebrtc::video_source::native::NativeVideoSource::new(
        VideoResolution {
            width: capture_width,
            height: capture_height,
        },
        true,
    );
    let video_track = LocalVideoTrack::create_video_track(
        "screen",
        RtcVideoSource::Native(video_source.clone()),
    );
    if let Err(e) = room.local_participant()
        .publish_track(
            LocalTrack::Video(video_track),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                simulcast: false,
                ..Default::default()
            },
        )
        .await
    {
        if let Some(tx) = ready_tx {
            let _ = tx.send(Err(format!("publish screen track failed: {e}")));
        }
        return Err(format!("publish screen track failed: {e}"));
    }

    info!(
        session_id = %session_id,
        room = %room_name,
        width = capture_width,
        height = capture_height,
        "connected to LiveKit room and published screen track"
    );

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|e| format!("failed to create input injector: {e:?}"))?;
    let mut input_state = InputState::default();
    let session_started_at = Instant::now();
    let mut input_event_count: u64 = 0;
    let mut last_input_at: Option<Instant> = None;
    let mut last_input_log_at: Option<Instant> = None;
    let mut last_idle_warn_at: Option<Instant> = None;

    if let Some(tx) = ready_tx {
        let _ = tx.send(Ok(()));
    }

    let capture_task = spawn_capture_loop(video_source.clone(), stop_flag.clone());

    while !stop_flag.load(Ordering::Relaxed) {
        match tokio::time::timeout(std::time::Duration::from_millis(250), room_events.recv()).await {
            Ok(Some(RoomEvent::DataReceived {
                payload,
                topic,
                ..
            })) if topic.as_deref() == Some("desktop-input") => {
                let now = Instant::now();
                apply_input_event(
                    &mut enigo,
                    &mut input_state,
                    &payload,
                    capture_width,
                    capture_height,
                );
                input_event_count += 1;
                last_input_at = Some(now);
                if last_input_log_at
                    .map(|at| now.duration_since(at) >= Duration::from_secs(10))
                    .unwrap_or(true)
                {
                    debug!(session_id = %session_id, room = %room_name, "desktop input channel active");
                    last_input_log_at = Some(now);
                }
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => {}
        }

        let now = Instant::now();
        if let Some(last_input) = last_input_at {
            if now.duration_since(last_input) >= Duration::from_secs(30)
                && last_idle_warn_at
                    .map(|at| now.duration_since(at) >= Duration::from_secs(30))
                    .unwrap_or(true)
            {
                warn!(
                    session_id = %session_id,
                    room = %room_name,
                    idle_for_secs = now.duration_since(last_input).as_secs(),
                    "desktop input channel idle"
                );
                last_idle_warn_at = Some(now);
            }
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    input_state.release_all(&mut enigo);
    let _ = capture_task.await;
    let _ = room.close().await;
    info!(
        session_id = %session_id,
        room = %room_name,
        session_uptime_secs = session_started_at.elapsed().as_secs(),
        input_event_count,
        last_input_age_secs = last_input_at.map(|at| at.elapsed().as_secs()),
        "desktop session input summary"
    );
    Ok(())
}
