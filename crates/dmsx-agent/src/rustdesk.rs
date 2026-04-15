use tracing::{info, warn};

pub(crate) fn configure_rustdesk_server(relay_server: &str) {
    let dir = match rustdesk_config_dir() {
        Some(d) => d,
        None => return,
    };
    let _ = std::fs::create_dir_all(&dir);

    let custom_file = dir.join("RustDesk2.toml");
    let content = format!(
        r#"[options]
custom-rendezvous-server = '{relay_server}'
relay-server = '{relay_server}'
api-server = ''
key = ''
"#
    );

    match std::fs::write(&custom_file, &content) {
        Ok(_) => info!(path = %custom_file.display(), "RustDesk server config written"),
        Err(e) => warn!("failed to write RustDesk config: {e}"),
    }
}

pub(crate) fn detect_rustdesk() -> serde_json::Value {
    let rustdesk_id = read_rustdesk_id();
    let has_password = read_rustdesk_password().is_some();
    let installed = rustdesk_id.is_some() || which_cmd("rustdesk");

    serde_json::json!({
        "installed": installed,
        "id": rustdesk_id,
        "has_permanent_password": has_password,
    })
}

fn rustdesk_config_dir() -> Option<std::path::PathBuf> {
    if cfg!(target_os = "windows") {
        std::env::var("APPDATA")
            .ok()
            .map(|p| std::path::PathBuf::from(p).join("RustDesk").join("config"))
    } else if cfg!(target_os = "macos") {
        dirs_fallback("Library/Preferences/com.carriez.RustDesk/config")
    } else {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/root".into());
        Some(std::path::PathBuf::from(home).join(".config/rustdesk"))
    }
}

fn dirs_fallback(suffix: &str) -> Option<std::path::PathBuf> {
    std::env::var("HOME")
        .ok()
        .map(|h| std::path::PathBuf::from(h).join(suffix))
}

fn read_rustdesk_id() -> Option<String> {
    let dir = rustdesk_config_dir()?;
    let id_file = dir.join("RustDesk.toml");
    if id_file.exists() {
        let content = std::fs::read_to_string(&id_file).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("id") {
                if let Some(val) = line.split('=').nth(1) {
                    let val = val.trim().trim_matches('\'').trim_matches('"');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    let id_file2 = dir.join("RustDesk2.toml");
    if id_file2.exists() {
        let content = std::fs::read_to_string(&id_file2).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("id") {
                if let Some(val) = line.split('=').nth(1) {
                    let val = val.trim().trim_matches('\'').trim_matches('"');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

fn read_rustdesk_password() -> Option<String> {
    let dir = rustdesk_config_dir()?;
    let id_file = dir.join("RustDesk.toml");
    if id_file.exists() {
        let content = std::fs::read_to_string(&id_file).ok()?;
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("password") {
                if let Some(val) = line.split('=').nth(1) {
                    let val = val.trim().trim_matches('\'').trim_matches('"');
                    if !val.is_empty() {
                        return Some(val.to_string());
                    }
                }
            }
        }
    }
    None
}

fn which_cmd(name: &str) -> bool {
    let check = if cfg!(target_os = "windows") {
        std::process::Command::new("where").arg(name).output()
    } else {
        std::process::Command::new("which").arg(name).output()
    };
    check.map(|o| o.status.success()).unwrap_or(false)
}
