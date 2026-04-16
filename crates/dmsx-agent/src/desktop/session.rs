use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

use tracing::{error, info};

use super::capture::{primary_capture_size, spawn_capture_loop};
use super::input::apply_input_event;

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

    let handle = tokio::spawn(async move {
        if let Err(e) = desktop_stream_loop(
            &livekit_url_owned,
            &token_owned,
            &room_owned,
            &session_id_owned,
            stop_clone,
        )
        .await
        {
            error!(session_id = %session_id_owned, "desktop session ended with error: {e}");
        }
        info!(session_id = %session_id_owned, "desktop session task exited");
    });

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
) -> Result<(), String> {
    use libwebrtc::prelude::{RtcVideoSource, VideoResolution};
    use livekit::options::TrackPublishOptions;
    use livekit::prelude::{LocalTrack, LocalVideoTrack, Room, RoomEvent, RoomOptions, TrackSource};

    let (capture_width, capture_height) = primary_capture_size().await?;

    let (room, mut room_events) = Room::connect(livekit_url, token, RoomOptions::default())
        .await
        .map_err(|e| format!("livekit connect failed: {e}"))?;

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
    room.local_participant()
        .publish_track(
            LocalTrack::Video(video_track),
            TrackPublishOptions {
                source: TrackSource::Screenshare,
                simulcast: false,
                ..Default::default()
            },
        )
        .await
        .map_err(|e| format!("publish screen track failed: {e}"))?;

    info!(
        session_id = %session_id,
        room = %room_name,
        width = capture_width,
        height = capture_height,
        "connected to LiveKit room and published screen track"
    );

    let capture_task = spawn_capture_loop(video_source.clone(), stop_flag.clone());

    let mut enigo = enigo::Enigo::new(&enigo::Settings::default())
        .map_err(|e| format!("failed to create input injector: {e:?}"))?;

    while !stop_flag.load(Ordering::Relaxed) {
        match tokio::time::timeout(std::time::Duration::from_millis(250), room_events.recv()).await {
            Ok(Some(RoomEvent::DataReceived {
                payload,
                topic,
                ..
            })) if topic.as_deref() == Some("desktop-input") => {
                apply_input_event(&mut enigo, &payload);
            }
            Ok(Some(_)) => {}
            Ok(None) => break,
            Err(_) => {}
        }
    }

    stop_flag.store(true, Ordering::Relaxed);
    let _ = capture_task.await;
    let _ = room.close().await;
    Ok(())
}
