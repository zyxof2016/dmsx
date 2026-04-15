use serde::Deserialize;

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

pub(super) fn apply_input_event(enigo: &mut enigo::Enigo, payload: &[u8]) {
    if let Ok(evt) = serde_json::from_slice::<InputEvent>(payload) {
        handle_input_event(enigo, &evt);
    }
}

fn handle_input_event(enigo: &mut enigo::Enigo, evt: &InputEvent) {
    use enigo::{Button, Coordinate, Direction, Keyboard, Mouse};

    match evt.event_type.as_str() {
        "mousemove" => {
            let _ = enigo.move_mouse(evt.x as i32, evt.y as i32, Coordinate::Abs);
        }
        "mousedown" => {
            let _ = enigo.move_mouse(evt.x as i32, evt.y as i32, Coordinate::Abs);
            let btn = match evt.button.as_str() {
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => Button::Left,
            };
            let _ = enigo.button(btn, Direction::Press);
        }
        "mouseup" => {
            let _ = enigo.move_mouse(evt.x as i32, evt.y as i32, Coordinate::Abs);
            let btn = match evt.button.as_str() {
                "right" => Button::Right,
                "middle" => Button::Middle,
                _ => Button::Left,
            };
            let _ = enigo.button(btn, Direction::Release);
        }
        "keydown" => {
            for modifier in &evt.modifiers {
                if let Some(key) = map_modifier(modifier) {
                    let _ = enigo.key(key, Direction::Press);
                }
            }
            if let Some(key) = map_key(&evt.key, &evt.code) {
                let _ = enigo.key(key, Direction::Press);
            }
        }
        "keyup" => {
            if let Some(key) = map_key(&evt.key, &evt.code) {
                let _ = enigo.key(key, Direction::Release);
            }
            for modifier in evt.modifiers.iter().rev() {
                if let Some(key) = map_modifier(modifier) {
                    let _ = enigo.key(key, Direction::Release);
                }
            }
        }
        "scroll" => {
            let _ = enigo.scroll(evt.delta_y as i32, enigo::Axis::Vertical);
        }
        _ => {}
    }
}

fn map_modifier(modifier: &str) -> Option<enigo::Key> {
    use enigo::Key;
    match modifier {
        "ctrl" => Some(Key::Control),
        "shift" => Some(Key::Shift),
        "alt" => Some(Key::Alt),
        "meta" => Some(Key::Meta),
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
