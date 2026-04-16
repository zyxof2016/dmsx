use uuid::Uuid;

use crate::dto::CreateCommandReq;

pub fn livekit_enabled(livekit_url: &str, livekit_api_key: &str) -> bool {
    !livekit_url.is_empty() && !livekit_api_key.is_empty()
}

pub fn build_start_desktop_command(
    device_id: Uuid,
    session_id: &str,
    room_name: &str,
    agent_token: &str,
    livekit_url: &str,
    width: u32,
    height: u32,
) -> CreateCommandReq {
    CreateCommandReq {
        target_device_id: device_id,
        payload: serde_json::json!({
            "action": "start_desktop",
            "params": {
                "room": room_name,
                "token": agent_token,
                "livekit_url": livekit_url,
                "session_id": session_id,
                "width": width,
                "height": height,
            }
        }),
        priority: Some(10),
        ttl_seconds: Some(120),
        idempotency_key: Some(format!("desktop-{session_id}")),
    }
}

pub fn build_stop_desktop_command(
    device_id: Uuid,
    session_id: &str,
    priority: Option<i16>,
) -> CreateCommandReq {
    CreateCommandReq {
        target_device_id: device_id,
        payload: serde_json::json!({
            "action": "stop_desktop",
            "params": {
                "session_id": session_id,
            }
        }),
        priority,
        ttl_seconds: Some(60),
        idempotency_key: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        build_start_desktop_command, build_stop_desktop_command, livekit_enabled,
    };
    use uuid::Uuid;

    #[test]
    fn livekit_enabled_requires_url_and_key() {
        assert!(livekit_enabled("ws://127.0.0.1:7880", "api-key"));
        assert!(!livekit_enabled("", "api-key"));
        assert!(!livekit_enabled("ws://127.0.0.1:7880", ""));
    }

    #[test]
    fn build_start_desktop_command_populates_expected_payload() {
        let device_id = Uuid::new_v4();
        let cmd = build_start_desktop_command(
            device_id,
            "session-1",
            "room-1",
            "agent-token",
            "ws://lk",
            1920,
            1080,
        );

        assert_eq!(cmd.target_device_id, device_id);
        assert_eq!(cmd.priority, Some(10));
        assert_eq!(cmd.ttl_seconds, Some(120));
        assert_eq!(cmd.idempotency_key.as_deref(), Some("desktop-session-1"));
        assert_eq!(cmd.payload["action"], "start_desktop");
        assert_eq!(cmd.payload["params"]["room"], "room-1");
        assert_eq!(cmd.payload["params"]["token"], "agent-token");
        assert_eq!(cmd.payload["params"]["livekit_url"], "ws://lk");
        assert_eq!(cmd.payload["params"]["session_id"], "session-1");
        assert_eq!(cmd.payload["params"]["width"], 1920);
        assert_eq!(cmd.payload["params"]["height"], 1080);
    }

    #[test]
    fn build_stop_desktop_command_populates_expected_payload() {
        let device_id = Uuid::new_v4();
        let cmd = build_stop_desktop_command(device_id, "session-2", Some(5));

        assert_eq!(cmd.target_device_id, device_id);
        assert_eq!(cmd.priority, Some(5));
        assert_eq!(cmd.ttl_seconds, Some(60));
        assert_eq!(cmd.idempotency_key, None);
        assert_eq!(cmd.payload["action"], "stop_desktop");
        assert_eq!(cmd.payload["params"]["session_id"], "session-2");
    }
}
