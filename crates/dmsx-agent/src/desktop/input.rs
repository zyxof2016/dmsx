use serde::Deserialize;
use std::collections::HashSet;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Deserialize)]
pub(super) struct InputEvent {
    #[serde(rename = "type")]
    event_type: String,
    #[serde(default)]
    x: f64,
    #[serde(default)]
    y: f64,
    #[serde(default)]
    button: String,
    #[serde(default)]
    key: String,
    #[serde(default)]
    code: String,
    #[serde(default)]
    #[allow(dead_code)]
    modifiers: Vec<String>,
    #[serde(rename = "deltaX", default)]
    #[allow(dead_code)]
    delta_x: f64,
    #[serde(rename = "deltaY", default)]
    delta_y: f64,
    #[serde(rename = "remoteWidth", default)]
    #[allow(dead_code)]
    remote_width: f64,
    #[serde(rename = "remoteHeight", default)]
    #[allow(dead_code)]
    remote_height: f64,
}

#[derive(Debug, Default)]
pub(super) struct InputState {
    pressed_modifiers: HashSet<ModifierKey>,
    pressed_keys: HashMap<String, PressedKey>,
    pressed_mouse_buttons: HashMap<MouseButtonKind, PressedMouseButton>,
}

impl InputState {
    pub fn release_all(&mut self, enigo: &mut enigo::Enigo) {
        use enigo::{Direction, Keyboard, Mouse};
        for (_id, pressed) in self.pressed_keys.drain() {
            let _ = enigo.key(pressed.key, Direction::Release);
        }
        for (_kind, pressed) in self.pressed_mouse_buttons.drain() {
            let _ = enigo.button(pressed.button, Direction::Release);
        }
        for modifier in self.pressed_modifiers.drain() {
            let _ = enigo.key(modifier.as_enigo_key(), Direction::Release);
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PressedKey {
    key: enigo::Key,
    pressed_at: Instant,
    last_seen_at: Instant,
}

const DEFAULT_STUCK_KEY_TIMEOUT: Duration = Duration::from_secs(3);
const DEFAULT_STUCK_MOUSE_BUTTON_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Clone, Copy, Debug)]
struct PressedMouseButton {
    button: enigo::Button,
    pressed_at: Instant,
    last_seen_at: Instant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum ModifierKey {
    Ctrl,
    Shift,
    Alt,
    Meta,
}

impl ModifierKey {
    fn as_enigo_key(self) -> enigo::Key {
        match self {
            ModifierKey::Ctrl => enigo::Key::Control,
            ModifierKey::Shift => enigo::Key::Shift,
            ModifierKey::Alt => enigo::Key::Alt,
            ModifierKey::Meta => enigo::Key::Meta,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
enum MouseButtonKind {
    Left,
    Middle,
    Right,
}

impl MouseButtonKind {
    fn as_enigo_button(self) -> enigo::Button {
        match self {
            MouseButtonKind::Left => enigo::Button::Left,
            MouseButtonKind::Middle => enigo::Button::Middle,
            MouseButtonKind::Right => enigo::Button::Right,
        }
    }
}

pub(super) fn apply_input_event(
    enigo: &mut enigo::Enigo,
    state: &mut InputState,
    payload: &[u8],
    local_width: u32,
    local_height: u32,
) {
    match serde_json::from_slice::<InputEvent>(payload) {
        Ok(evt) => handle_input_event(enigo, state, &evt, local_width, local_height),
        Err(err) => {
            tracing::debug!(
                payload_len = payload.len(),
                error = %err,
                "failed to decode desktop input event"
            );
        }
    }
}

fn handle_input_event(
    enigo: &mut enigo::Enigo,
    state: &mut InputState,
    evt: &InputEvent,
    local_width: u32,
    local_height: u32,
) {
    use enigo::{Button, Coordinate, Direction, Keyboard, Mouse};

    let now = Instant::now();
    let key_timeout = env_timeout_seconds("DMSX_AGENT_DESKTOP_STUCK_KEY_TIMEOUT_SECONDS")
        .unwrap_or(DEFAULT_STUCK_KEY_TIMEOUT);
    let mouse_timeout = env_timeout_seconds("DMSX_AGENT_DESKTOP_STUCK_MOUSE_TIMEOUT_SECONDS")
        .unwrap_or(DEFAULT_STUCK_MOUSE_BUTTON_TIMEOUT);
    release_stuck_keys(enigo, state, now, key_timeout);
    release_stuck_mouse_buttons(enigo, state, now, mouse_timeout);

    match evt.event_type.as_str() {
        "mousemove" => {
            sync_modifier_state(enigo, state, &evt.modifiers);
            touch_pressed_mouse_buttons(state, now);
            if let Some((x, y)) = scaled_clamped_xy(evt, local_width, local_height) {
                let _ = enigo.move_mouse(x, y, Coordinate::Abs);
            }
        }
        "mousedown" => {
            sync_modifier_state(enigo, state, &evt.modifiers);
            if let Some((x, y)) = scaled_clamped_xy(evt, local_width, local_height) {
                let _ = enigo.move_mouse(x, y, Coordinate::Abs);
            }
            if let Some(kind) = map_mouse_button_kind(&evt.button) {
                let now = Instant::now();
                if let Some(existing) = state.pressed_mouse_buttons.get_mut(&kind) {
                    existing.last_seen_at = now;
                } else {
                    let btn = kind.as_enigo_button();
                    let _ = enigo.button(btn, Direction::Press);
                    state.pressed_mouse_buttons.insert(
                        kind,
                        PressedMouseButton {
                            button: btn,
                            pressed_at: now,
                            last_seen_at: now,
                        },
                    );
                }
            } else {
                let btn = match evt.button.as_str() {
                    "right" => Button::Right,
                    "middle" => Button::Middle,
                    _ => Button::Left,
                };
                let _ = enigo.button(btn, Direction::Press);
            }
        }
        "mouseup" => {
            sync_modifier_state(enigo, state, &evt.modifiers);
            if let Some((x, y)) = scaled_clamped_xy(evt, local_width, local_height) {
                let _ = enigo.move_mouse(x, y, Coordinate::Abs);
            }
            if let Some(kind) = map_mouse_button_kind(&evt.button) {
                let btn = kind.as_enigo_button();
                let _ = enigo.button(btn, Direction::Release);
                state.pressed_mouse_buttons.remove(&kind);
            } else {
                let btn = match evt.button.as_str() {
                    "right" => Button::Right,
                    "middle" => Button::Middle,
                    _ => Button::Left,
                };
                let _ = enigo.button(btn, Direction::Release);
            }
        }
        "keydown" => {
            sync_modifier_state(enigo, state, &evt.modifiers);
            if let Some((id, key)) = key_id_and_key(evt) {
                if !is_modifier_enigo_key(&key) {
                    let now = Instant::now();
                    if let Some(existing) = state.pressed_keys.get_mut(&id) {
                        existing.last_seen_at = now;
                    } else {
                        let _ = enigo.key(key, Direction::Press);
                        state.pressed_keys.insert(
                            id,
                            PressedKey {
                                key,
                                pressed_at: now,
                                last_seen_at: now,
                            },
                        );
                    }
                }
            }
        }
        "keyup" => {
            if let Some((id, key)) = key_id_and_key(evt) {
                if !is_modifier_enigo_key(&key) {
                    let _ = enigo.key(key, Direction::Release);
                    state.pressed_keys.remove(&id);
                }
            }
            sync_modifier_state(enigo, state, &evt.modifiers);
        }
        "scroll" => {
            sync_modifier_state(enigo, state, &evt.modifiers);
            let dx = normalized_scroll_delta(evt.delta_x);
            let dy = normalized_scroll_delta(evt.delta_y);
            if dx != 0 {
                let _ = enigo.scroll(dx, enigo::Axis::Horizontal);
            }
            if dy != 0 {
                let _ = enigo.scroll(dy, enigo::Axis::Vertical);
            }
        }
        other => {
            tracing::trace!(event_type = other, "ignored unknown desktop input event type");
        }
    }
}

fn sync_modifier_state(enigo: &mut enigo::Enigo, state: &mut InputState, modifiers: &[String]) {
    use enigo::{Direction, Keyboard};

    let desired = desired_modifiers(modifiers);
    for modifier in state.pressed_modifiers.difference(&desired).copied().collect::<Vec<_>>() {
        let _ = enigo.key(modifier.as_enigo_key(), Direction::Release);
        state.pressed_modifiers.remove(&modifier);
    }
    for modifier in desired.difference(&state.pressed_modifiers).copied().collect::<Vec<_>>() {
        let _ = enigo.key(modifier.as_enigo_key(), Direction::Press);
        state.pressed_modifiers.insert(modifier);
    }
}

fn release_stuck_keys(
    enigo: &mut enigo::Enigo,
    state: &mut InputState,
    now: Instant,
    timeout: Duration,
) {
    use enigo::{Direction, Keyboard};

    let stuck = collect_stuck_key_ids(&state.pressed_keys, now, timeout);
    for id in stuck {
        if let Some(pressed) = state.pressed_keys.remove(&id) {
            tracing::debug!(
                key_id = id,
                pressed_for_ms = pressed.pressed_at.elapsed().as_millis() as u64,
                "releasing stuck key"
            );
            let _ = enigo.key(pressed.key, Direction::Release);
        }
    }
}

fn collect_stuck_key_ids(
    pressed: &HashMap<String, PressedKey>,
    now: Instant,
    timeout: Duration,
) -> Vec<String> {
    pressed
        .iter()
        .filter_map(|(id, key)| {
            if now.duration_since(key.last_seen_at) >= timeout {
                Some(id.clone())
            } else {
                None
            }
        })
        .collect()
}

fn release_stuck_mouse_buttons(
    enigo: &mut enigo::Enigo,
    state: &mut InputState,
    now: Instant,
    timeout: Duration,
) {
    use enigo::{Direction, Mouse};

    let stuck = collect_stuck_mouse_buttons(&state.pressed_mouse_buttons, now, timeout);
    for kind in stuck {
        if let Some(pressed) = state.pressed_mouse_buttons.remove(&kind) {
            tracing::debug!(
                mouse_button = ?kind,
                pressed_for_ms = pressed.pressed_at.elapsed().as_millis() as u64,
                "releasing stuck mouse button"
            );
            let _ = enigo.button(pressed.button, Direction::Release);
        }
    }
}

fn collect_stuck_mouse_buttons(
    pressed: &HashMap<MouseButtonKind, PressedMouseButton>,
    now: Instant,
    timeout: Duration,
) -> Vec<MouseButtonKind> {
    pressed
        .iter()
        .filter_map(|(kind, button)| {
            if now.duration_since(button.last_seen_at) >= timeout {
                Some(*kind)
            } else {
                None
            }
        })
        .collect()
}

fn touch_pressed_mouse_buttons(state: &mut InputState, now: Instant) {
    for button in state.pressed_mouse_buttons.values_mut() {
        button.last_seen_at = now;
    }
}

fn env_timeout_seconds(name: &str) -> Option<Duration> {
    let raw = std::env::var(name).ok()?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    let secs: u64 = trimmed.parse().ok()?;
    Some(Duration::from_secs(secs))
}

fn key_id_and_key(evt: &InputEvent) -> Option<(String, enigo::Key)> {
    let key = map_key(&evt.key, &evt.code)?;
    let id = if !evt.code.trim().is_empty() {
        format!("code:{}", evt.code.trim())
    } else if let Some(ch) = evt.key.chars().next() {
        format!("char:{:04X}", ch as u32)
    } else {
        return None;
    };
    Some((id, key))
}

fn desired_modifiers(modifiers: &[String]) -> HashSet<ModifierKey> {
    modifiers
        .iter()
        .filter_map(|modifier| map_modifier(modifier))
        .collect()
}

fn is_modifier_enigo_key(key: &enigo::Key) -> bool {
    matches!(
        key,
        enigo::Key::Control | enigo::Key::Shift | enigo::Key::Alt | enigo::Key::Meta
    )
}

fn scaled_clamped_xy(evt: &InputEvent, local_width: u32, local_height: u32) -> Option<(i32, i32)> {
    if local_width == 0 || local_height == 0 {
        return None;
    }

    let (mut x, mut y) = (evt.x, evt.y);
    if evt.remote_width.is_finite() && evt.remote_height.is_finite() && evt.remote_width > 0.0 && evt.remote_height > 0.0 {
        x = x * (local_width as f64) / evt.remote_width;
        y = y * (local_height as f64) / evt.remote_height;
    }

    if !x.is_finite() || !y.is_finite() {
        return None;
    }

    let max_x = (local_width - 1) as f64;
    let max_y = (local_height - 1) as f64;
    let xi = x.round().clamp(0.0, max_x) as i32;
    let yi = y.round().clamp(0.0, max_y) as i32;
    Some((xi, yi))
}

fn clamp_i32_from_f64(value: f64) -> i32 {
    if !value.is_finite() {
        return 0;
    }
    if value >= i32::MAX as f64 {
        return i32::MAX;
    }
    if value <= i32::MIN as f64 {
        return i32::MIN;
    }
    value.round() as i32
}

fn normalized_scroll_delta(value: f64) -> i32 {
    // Touchpads/browsers can emit huge deltas. Clamp to keep behavior controllable.
    const MAX_ABS_DELTA: i32 = 120;
    let delta = clamp_i32_from_f64(value);
    delta.clamp(-MAX_ABS_DELTA, MAX_ABS_DELTA)
}

fn map_modifier(modifier: &str) -> Option<ModifierKey> {
    match modifier {
        "ctrl" => Some(ModifierKey::Ctrl),
        "shift" => Some(ModifierKey::Shift),
        "alt" => Some(ModifierKey::Alt),
        "meta" => Some(ModifierKey::Meta),
        _ => None,
    }
}

fn map_mouse_button_kind(button: &str) -> Option<MouseButtonKind> {
    match button {
        "left" | "" => Some(MouseButtonKind::Left),
        "middle" => Some(MouseButtonKind::Middle),
        "right" => Some(MouseButtonKind::Right),
        _ => None,
    }
}

fn map_key(key: &str, code: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match code {
        "Backspace" => Some(Key::Backspace),
        "Tab" => Some(Key::Tab),
        "Enter" | "NumpadEnter" => Some(Key::Return),
        "ShiftLeft" | "ShiftRight" => Some(Key::Shift),
        "ControlLeft" | "ControlRight" => Some(Key::Control),
        "AltLeft" | "AltRight" => Some(Key::Alt),
        "Escape" => Some(Key::Escape),
        "Space" => Some(Key::Space),
        "ArrowUp" => Some(Key::UpArrow),
        "ArrowDown" => Some(Key::DownArrow),
        "ArrowLeft" => Some(Key::LeftArrow),
        "ArrowRight" => Some(Key::RightArrow),
        "Delete" => Some(Key::Delete),
        "Home" => Some(Key::Home),
        "End" => Some(Key::End),
        "PageUp" => Some(Key::PageUp),
        "PageDown" => Some(Key::PageDown),
        "CapsLock" => Some(Key::CapsLock),
        "MetaLeft" | "MetaRight" => Some(Key::Meta),
        "F1" => Some(Key::F1),
        "F2" => Some(Key::F2),
        "F3" => Some(Key::F3),
        "F4" => Some(Key::F4),
        "F5" => Some(Key::F5),
        "F6" => Some(Key::F6),
        "F7" => Some(Key::F7),
        "F8" => Some(Key::F8),
        "F9" => Some(Key::F9),
        "F10" => Some(Key::F10),
        "F11" => Some(Key::F11),
        "F12" => Some(Key::F12),
        _ => {
            let ch = key.chars().next()?;
            Some(Key::Unicode(ch))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scaled_clamped_xy_scales_from_remote_dimensions() {
        let evt = InputEvent {
            event_type: "mousemove".to_string(),
            x: 50.0,
            y: 25.0,
            button: "".to_string(),
            key: "".to_string(),
            code: "".to_string(),
            modifiers: Vec::new(),
            delta_x: 0.0,
            delta_y: 0.0,
            remote_width: 100.0,
            remote_height: 50.0,
        };

        let (x, y) = scaled_clamped_xy(&evt, 1920, 1080).expect("xy");
        assert_eq!(x, 960);
        assert_eq!(y, 540);
    }

    #[test]
    fn scaled_clamped_xy_clamps_and_rejects_nan() {
        let evt = InputEvent {
            event_type: "mousemove".to_string(),
            x: f64::NAN,
            y: 10.0,
            button: "".to_string(),
            key: "".to_string(),
            code: "".to_string(),
            modifiers: Vec::new(),
            delta_x: 0.0,
            delta_y: 0.0,
            remote_width: 0.0,
            remote_height: 0.0,
        };
        assert!(scaled_clamped_xy(&evt, 800, 600).is_none());

        let evt2 = InputEvent { x: -100.0, y: 9999.0, ..evt };
        let (x2, y2) = scaled_clamped_xy(&evt2, 800, 600).expect("xy");
        assert_eq!(x2, 0);
        assert_eq!(y2, 599);
    }

    #[test]
    fn desired_modifiers_dedupes_and_maps_known_modifiers() {
        let modifiers = vec![
            "ctrl".to_string(),
            "ctrl".to_string(),
            "shift".to_string(),
            "unknown".to_string(),
        ];
        let set = desired_modifiers(&modifiers);
        assert!(set.contains(&ModifierKey::Ctrl));
        assert!(set.contains(&ModifierKey::Shift));
        assert_eq!(set.len(), 2);
    }

    #[test]
    fn collect_stuck_key_ids_finds_keys_past_timeout() {
        let now = Instant::now();
        let mut pressed = HashMap::new();
        pressed.insert(
            "a".to_string(),
            PressedKey {
                key: enigo::Key::Unicode('a'),
                pressed_at: now - Duration::from_secs(10),
                last_seen_at: now - Duration::from_secs(10),
            },
        );
        pressed.insert(
            "b".to_string(),
            PressedKey {
                key: enigo::Key::Unicode('b'),
                pressed_at: now - Duration::from_secs(1),
                last_seen_at: now - Duration::from_secs(1),
            },
        );

        let stuck = collect_stuck_key_ids(&pressed, now, Duration::from_secs(3));
        assert_eq!(stuck, vec!["a".to_string()]);
    }

    #[test]
    fn collect_stuck_mouse_buttons_finds_buttons_past_timeout() {
        let now = Instant::now();
        let mut pressed = HashMap::new();
        pressed.insert(
            MouseButtonKind::Left,
            PressedMouseButton {
                button: enigo::Button::Left,
                pressed_at: now - Duration::from_secs(10),
                last_seen_at: now - Duration::from_secs(10),
            },
        );
        pressed.insert(
            MouseButtonKind::Right,
            PressedMouseButton {
                button: enigo::Button::Right,
                pressed_at: now - Duration::from_secs(1),
                last_seen_at: now - Duration::from_secs(1),
            },
        );

        let stuck = collect_stuck_mouse_buttons(&pressed, now, Duration::from_secs(3));
        assert_eq!(stuck, vec![MouseButtonKind::Left]);
    }

    #[test]
    fn env_timeout_seconds_returns_none_for_missing_or_invalid() {
        std::env::remove_var("DMSX_AGENT_DESKTOP_TEST_TIMEOUT");
        assert!(env_timeout_seconds("DMSX_AGENT_DESKTOP_TEST_TIMEOUT").is_none());

        std::env::set_var("DMSX_AGENT_DESKTOP_TEST_TIMEOUT", "nope");
        assert!(env_timeout_seconds("DMSX_AGENT_DESKTOP_TEST_TIMEOUT").is_none());

        std::env::set_var("DMSX_AGENT_DESKTOP_TEST_TIMEOUT", "5");
        assert_eq!(
            env_timeout_seconds("DMSX_AGENT_DESKTOP_TEST_TIMEOUT"),
            Some(Duration::from_secs(5))
        );
        std::env::remove_var("DMSX_AGENT_DESKTOP_TEST_TIMEOUT");
    }

    #[test]
    fn normalized_scroll_delta_clamps_large_values() {
        assert_eq!(normalized_scroll_delta(0.0), 0);
        assert_eq!(normalized_scroll_delta(1.4), 1);
        assert_eq!(normalized_scroll_delta(120.0), 120);
        assert_eq!(normalized_scroll_delta(9999.0), 120);
        assert_eq!(normalized_scroll_delta(-9999.0), -120);
        assert_eq!(normalized_scroll_delta(f64::NAN), 0);
    }
}
