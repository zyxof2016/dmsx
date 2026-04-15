use std::io::Cursor;
use std::process::Stdio;
use std::sync::Arc;

use chrono::Utc;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use sysinfo::{Disks, System};
use tokio::process::Command as TokioCommand;
use tokio::sync::Notify;
use tracing::{error, info, warn};

// ---------------------------------------------------------------------------
// Config
// ---------------------------------------------------------------------------

struct AgentConfig {
    api_base: String,
    tenant_id: String,
    heartbeat_interval: std::time::Duration,
    command_poll_interval: std::time::Duration,
    rustdesk_relay: Option<String>,
}

impl AgentConfig {
    fn from_env() -> Self {
        Self {
            api_base: std::env::var("DMSX_API_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            tenant_id: std::env::var("DMSX_TENANT_ID")
                .unwrap_or_else(|_| "00000000-0000-0000-0000-000000000001".into()),
            heartbeat_interval: std::time::Duration::from_secs(
                std::env::var("DMSX_HEARTBEAT_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(30),
            ),
            command_poll_interval: std::time::Duration::from_secs(
                std::env::var("DMSX_POLL_SECS")
                    .ok()
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(10),
            ),
            rustdesk_relay: std::env::var("DMSX_RUSTDESK_RELAY").ok(),
        }
    }

    fn tenant_url(&self, path: &str) -> String {
        format!("{}/v1/tenants/{}{}", self.api_base, self.tenant_id, path)
    }
}

// ---------------------------------------------------------------------------
// API types (subset mirroring backend)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct CreateDeviceReq {
    platform: String,
    hostname: Option<String>,
    os_version: Option<String>,
    agent_version: Option<String>,
    labels: serde_json::Value,
}

#[derive(Debug, Deserialize)]
struct Device {
    id: String,
    hostname: Option<String>,
    online_state: String,
}

#[derive(Debug, Deserialize)]
struct ListResponse<T> {
    items: Vec<T>,
}

#[derive(Debug, Deserialize)]
struct CommandItem {
    id: String,
    status: String,
    payload: serde_json::Value,
}

#[derive(Debug, Serialize)]
struct SubmitResultReq {
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[derive(Debug, Serialize)]
struct UpdateStatusReq {
    status: String,
}

// ---------------------------------------------------------------------------
// System telemetry
// ---------------------------------------------------------------------------

fn detect_platform() -> &'static str {
    if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "android") {
        "android"
    } else if cfg!(target_os = "linux") {
        if is_android_runtime() { "android" } else { "linux" }
    } else {
        "other"
    }
}

fn is_android_runtime() -> bool {
    std::path::Path::new("/system/build.prop").exists()
        || std::env::var("ANDROID_ROOT").is_ok()
        || std::env::var("PREFIX")
            .map(|p| p.contains("com.termux"))
            .unwrap_or(false)
}

fn collect_telemetry() -> serde_json::Value {
    let mut sys = System::new_all();
    sys.refresh_all();

    let disks = Disks::new_with_refreshed_list();
    let disk_info: Vec<serde_json::Value> = disks
        .list()
        .iter()
        .map(|d| {
            serde_json::json!({
                "mount": d.mount_point().to_string_lossy(),
                "total_gb": d.total_space() as f64 / 1_073_741_824.0,
                "free_gb": d.available_space() as f64 / 1_073_741_824.0,
            })
        })
        .collect();

    let mut telemetry = serde_json::json!({
        "agent_version": env!("CARGO_PKG_VERSION"),
        "platform": detect_platform(),
        "os_name": System::name().unwrap_or_default(),
        "os_version": System::os_version().unwrap_or_default(),
        "kernel_version": System::kernel_version().unwrap_or_default(),
        "hostname": System::host_name().unwrap_or_default(),
        "cpu_count": sys.cpus().len(),
        "cpu_brand": sys.cpus().first().map(|c| c.brand().to_string()).unwrap_or_default(),
        "total_memory_mb": sys.total_memory() / 1_048_576,
        "used_memory_mb": sys.used_memory() / 1_048_576,
        "total_swap_mb": sys.total_swap() / 1_048_576,
        "uptime_secs": System::uptime(),
        "disks": disk_info,
        "collected_at": Utc::now().to_rfc3339(),
    });

    if detect_platform() == "android" {
        if let Some(android_info) = collect_android_props() {
            telemetry.as_object_mut().unwrap().insert(
                "android".into(),
                android_info,
            );
        }
    }

    let rd = detect_rustdesk();
    telemetry.as_object_mut().unwrap().insert("rustdesk".into(), rd);

    telemetry
}

fn collect_android_props() -> Option<serde_json::Value> {
    let read_prop = |key: &str| -> String {
        std::process::Command::new("getprop")
            .arg(key)
            .output()
            .ok()
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .unwrap_or_default()
    };

    let model = read_prop("ro.product.model");
    if model.is_empty() {
        return None;
    }

    Some(serde_json::json!({
        "model": model,
        "manufacturer": read_prop("ro.product.manufacturer"),
        "brand": read_prop("ro.product.brand"),
        "sdk_version": read_prop("ro.build.version.sdk"),
        "android_version": read_prop("ro.build.version.release"),
        "build_fingerprint": read_prop("ro.build.fingerprint"),
        "security_patch": read_prop("ro.build.version.security_patch"),
        "serial": read_prop("ro.serialno"),
        "abi": read_prop("ro.product.cpu.abi"),
    }))
}

// ---------------------------------------------------------------------------
// RustDesk integration — detect ID, configure relay server
// ---------------------------------------------------------------------------

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

fn configure_rustdesk_server(relay_server: &str) {
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

fn detect_rustdesk() -> serde_json::Value {
    let rustdesk_id = read_rustdesk_id();
    let has_password = read_rustdesk_password().is_some();
    let installed = rustdesk_id.is_some()
        || which_cmd("rustdesk");

    serde_json::json!({
        "installed": installed,
        "id": rustdesk_id,
        "has_permanent_password": has_password,
    })
}

fn which_cmd(name: &str) -> bool {
    let check = if cfg!(target_os = "windows") {
        std::process::Command::new("where").arg(name).output()
    } else {
        std::process::Command::new("which").arg(name).output()
    };
    check.map(|o| o.status.success()).unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Device registration
// ---------------------------------------------------------------------------

async fn find_or_register_device(
    client: &Client,
    cfg: &AgentConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let hostname = System::host_name().unwrap_or_else(|| "unknown".into());
    let platform = detect_platform();

    let url = cfg.tenant_url(&format!(
        "/devices?search={}&platform={}&limit=5",
        hostname, platform
    ));
    let resp: ListResponse<Device> = client.get(&url).send().await?.json().await?;

    if let Some(existing) = resp.items.into_iter().find(|d| {
        d.hostname.as_deref() == Some(hostname.as_str())
    }) {
        info!(device_id = %existing.id, "found existing device registration");
        return Ok(existing.id);
    }

    info!("no existing device found, registering new device...");
    let body = CreateDeviceReq {
        platform: platform.into(),
        hostname: Some(hostname.clone()),
        os_version: System::os_version(),
        agent_version: Some(env!("CARGO_PKG_VERSION").into()),
        labels: serde_json::json!({
            "agent": "dmsx-agent",
            "auto_registered": true,
            "registered_at": Utc::now().to_rfc3339(),
        }),
    };

    let resp = client
        .post(cfg.tenant_url("/devices"))
        .json(&body)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("register failed: {status} — {text}").into());
    }

    let dev: Device = resp.json().await?;
    info!(device_id = %dev.id, hostname = %hostname, "device registered successfully");
    Ok(dev.id)
}

// ---------------------------------------------------------------------------
// Shadow heartbeat (update reported state with system telemetry)
// ---------------------------------------------------------------------------

async fn heartbeat(
    client: &Client,
    cfg: &AgentConfig,
    device_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let telemetry = collect_telemetry();

    let device_url = cfg.tenant_url(&format!("/devices/{device_id}"));
    let _ = client
        .patch(&device_url)
        .json(&serde_json::json!({
            "online_state": "online",
            "agent_version": env!("CARGO_PKG_VERSION"),
            "os_version": System::os_version(),
        }))
        .send()
        .await;

    // Also update shadow reported state via GET (create) + internal API
    // Since we don't have a direct PATCH reported endpoint exposed in REST,
    // we'll call GET to ensure shadow exists, then the telemetry is available
    // when admin views the shadow panel.
    // For a full implementation, we'd add PATCH .../shadow/reported endpoint.
    // For now, let's add it:
    let shadow_reported_url = cfg.tenant_url(&format!("/devices/{device_id}/shadow/reported"));
    let resp = client
        .patch(&shadow_reported_url)
        .json(&serde_json::json!({ "reported": telemetry }))
        .send()
        .await;

    match resp {
        Ok(r) if r.status().is_success() => {
            info!("heartbeat sent — shadow reported updated");
        }
        Ok(r) => {
            // Might be 404 if reported endpoint not yet available, fall back to just device update
            let _ = r.status();
            info!("heartbeat sent — device state updated (shadow reported endpoint not available)");
        }
        Err(e) => warn!("heartbeat shadow update failed: {e}"),
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Command polling + execution
// ---------------------------------------------------------------------------

async fn poll_and_execute(
    client: &Client,
    cfg: &AgentConfig,
    device_id: &str,
    desktop_session: &mut Option<DesktopSession>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = cfg.tenant_url(&format!(
        "/devices/{device_id}/commands?limit=10"
    ));
    let resp: ListResponse<CommandItem> = client.get(&url).send().await?.json().await?;

    let queued: Vec<&CommandItem> = resp
        .items
        .iter()
        .filter(|c| c.status == "queued")
        .collect();

    if queued.is_empty() {
        return Ok(());
    }

    info!(count = queued.len(), "found queued commands");

    for cmd in queued {
        execute_command(client, cfg, cmd, desktop_session).await;
    }

    Ok(())
}

async fn execute_command(
    client: &Client,
    cfg: &AgentConfig,
    cmd: &CommandItem,
    desktop_session: &mut Option<DesktopSession>,
) {
    let cmd_id = &cmd.id;
    let action = cmd.payload.get("action").and_then(|v| v.as_str()).unwrap_or("unknown");
    let params = cmd.payload.get("params").cloned().unwrap_or(serde_json::json!({}));

    info!(command_id = %cmd_id, action = %action, "executing command");

    let status_url = cfg.tenant_url(&format!("/commands/{cmd_id}/status"));
    let _ = client
        .patch(&status_url)
        .json(&UpdateStatusReq { status: "running".into() })
        .send()
        .await;

    let (exit_code, stdout, stderr) = match action {
        "start_desktop" => {
            if desktop_session.is_some() {
                info!("stopping existing desktop session before starting new one");
                if let Some(session) = desktop_session.take() {
                    session.stop_signal.notify_one();
                    let _ = session.handle.await;
                }
            }
            match start_desktop_session(cfg, &cmd.payload.get("target_device_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown"), &params).await
            {
                Ok(session) => {
                    *desktop_session = Some(session);
                    (0, "desktop session started".into(), String::new())
                }
                Err(e) => {
                    error!("failed to start desktop session: {e}");
                    (1, String::new(), format!("start_desktop failed: {e}"))
                }
            }
        }
        "stop_desktop" => {
            if let Some(session) = desktop_session.take() {
                session.stop_signal.notify_one();
                let _ = session.handle.await;
                (0, "desktop session stopped".into(), String::new())
            } else {
                (0, "no active desktop session".into(), String::new())
            }
        }
        "run_script" => run_script(&params).await,
        "reboot" => {
            info!("reboot requested — scheduling system reboot");
            schedule_reboot().await
        }
        "shutdown" => {
            let delay = params.get("delay_seconds").and_then(|v| v.as_u64()).unwrap_or(30);
            info!(delay_seconds = delay, "shutdown requested");
            schedule_shutdown(delay).await
        }
        "lock_screen" => {
            info!("lock_screen requested");
            lock_screen().await
        }
        "collect_logs" => {
            info!("collecting system logs");
            collect_logs(&params).await
        }
        "wipe" => {
            warn!("WIPE command received — refusing in agent (safety)");
            (1, String::new(), "wipe refused by agent safety policy".into())
        }
        "install_update" => {
            info!("install_update requested");
            (0, "update acknowledged (stub)".into(), String::new())
        }
        _ => {
            warn!(action = %action, "unknown action");
            (1, String::new(), format!("unknown action: {action}"))
        }
    };

    let result_url = cfg.tenant_url(&format!("/commands/{cmd_id}/result"));
    let result_body = SubmitResultReq {
        exit_code: Some(exit_code),
        stdout,
        stderr,
    };

    match client.post(&result_url).json(&result_body).send().await {
        Ok(r) if r.status().is_success() => {
            info!(command_id = %cmd_id, exit_code, "command result submitted");
        }
        Ok(r) => {
            let st = r.status();
            let body = r.text().await.unwrap_or_default();
            error!(command_id = %cmd_id, status = %st, "failed to submit result: {body}");
        }
        Err(e) => error!(command_id = %cmd_id, "failed to submit result: {e}"),
    }
}

async fn run_script(params: &serde_json::Value) -> (i32, String, String) {
    let script = match params.get("script").and_then(|v| v.as_str()) {
        Some(s) => s,
        None => return (1, String::new(), "missing script parameter".into()),
    };
    let interpreter = params
        .get("interpreter")
        .and_then(|v| v.as_str())
        .unwrap_or(if cfg!(target_os = "windows") { "powershell" } else { "bash" });
    let timeout_secs = params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(60);

    let (program, args): (&str, Vec<&str>) = match interpreter {
        "powershell" | "pwsh" => {
            if cfg!(target_os = "windows") {
                ("powershell.exe", vec!["-NoProfile", "-Command", script])
            } else {
                ("pwsh", vec!["-NoProfile", "-Command", script])
            }
        }
        "bash" => ("bash", vec!["-c", script]),
        "sh" => ("sh", vec!["-c", script]),
        "python" | "python3" => ("python3", vec!["-c", script]),
        other => return (1, String::new(), format!("unsupported interpreter: {other}")),
    };

    info!(interpreter, timeout_secs, "running script");

    let child = TokioCommand::new(program)
        .args(&args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn();

    let child = match child {
        Ok(c) => c,
        Err(e) => return (1, String::new(), format!("spawn failed: {e}")),
    };

    match tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        child.wait_with_output(),
    )
    .await
    {
        Ok(Ok(output)) => {
            let code = output.status.code().unwrap_or(-1);
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            (code, stdout, stderr)
        }
        Ok(Err(e)) => (1, String::new(), format!("process error: {e}")),
        Err(_) => {
            (124, String::new(), format!("timeout after {timeout_secs}s"))
        }
    }
}

async fn schedule_reboot() -> (i32, String, String) {
    let (prog, args): (&str, &[&str]) = if cfg!(target_os = "windows") {
        ("shutdown.exe", &["/r", "/t", "5", "/c", "DMSX remote reboot"])
    } else {
        ("sudo", &["shutdown", "-r", "+1", "DMSX remote reboot"])
    };
    run_system_command(prog, args).await
}

async fn schedule_shutdown(delay: u64) -> (i32, String, String) {
    let delay_str = delay.to_string();
    let mins = format!("+{}", (delay / 60).max(1));
    let (prog, args): (&str, Vec<&str>) = if cfg!(target_os = "windows") {
        ("shutdown.exe", vec!["/s", "/t", &delay_str, "/c", "DMSX remote shutdown"])
    } else {
        ("sudo", vec!["shutdown", "-h", &mins, "DMSX remote shutdown"])
    };
    run_system_command(prog, &args).await
}

async fn lock_screen() -> (i32, String, String) {
    if cfg!(target_os = "windows") {
        run_system_command("rundll32.exe", &["user32.dll,LockWorkStation"]).await
    } else if cfg!(target_os = "macos") {
        run_system_command(
            "osascript",
            &["-e", r#"tell application "System Events" to keystroke "q" using {control down, command down}"#],
        )
        .await
    } else {
        run_system_command("loginctl", &["lock-session"]).await
    }
}

async fn collect_logs(params: &serde_json::Value) -> (i32, String, String) {
    let log_types: Vec<&str> = params
        .get("log_types")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_else(|| vec!["system"]);

    let mut all_stdout = String::new();

    for lt in &log_types {
        all_stdout.push_str(&format!("=== {lt} logs ===\n"));
        let (_, out, _) = if cfg!(target_os = "windows") {
            match *lt {
                "system" => {
                    run_system_command(
                        "powershell.exe",
                        &["-NoProfile", "-Command", "Get-EventLog -LogName System -Newest 50 | Format-Table -AutoSize"],
                    ).await
                }
                "agent" => (0, "(agent log collection not implemented yet)".into(), String::new()),
                _ => (0, format!("(unknown log type: {lt})"), String::new()),
            }
        } else {
            match *lt {
                "system" => run_system_command("journalctl", &["-n", "100", "--no-pager"]).await,
                "agent" => (0, "(agent log collection not implemented yet)".into(), String::new()),
                _ => (0, format!("(unknown log type: {lt})"), String::new()),
            }
        };
        all_stdout.push_str(&out);
        all_stdout.push('\n');
    }

    (0, all_stdout, String::new())
}

async fn run_system_command(program: &str, args: &[&str]) -> (i32, String, String) {
    match TokioCommand::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .await
    {
        Ok(output) => {
            let code = output.status.code().unwrap_or(-1);
            (
                code,
                String::from_utf8_lossy(&output.stdout).to_string(),
                String::from_utf8_lossy(&output.stderr).to_string(),
            )
        }
        Err(e) => (1, String::new(), format!("failed to run {program}: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Remote Desktop — screen capture + input injection via WebSocket
// ---------------------------------------------------------------------------

struct DesktopSession {
    stop_signal: Arc<Notify>,
    handle: tokio::task::JoinHandle<()>,
}

#[derive(Debug, Deserialize)]
struct InputEvent {
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

async fn start_desktop_session(
    cfg: &AgentConfig,
    _device_id: &str,
    params: &serde_json::Value,
) -> Result<DesktopSession, String> {
    let ws_path = params
        .get("api_ws_url")
        .and_then(|v| v.as_str())
        .ok_or("missing api_ws_url in params")?;

    let api_host = cfg
        .api_base
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let ws_url = format!("ws://{}{}", api_host, ws_path);

    info!(ws_url = %ws_url, "starting desktop session");

    let stop = Arc::new(Notify::new());
    let stop_clone = stop.clone();
    let ws_url_owned = ws_url.clone();

    let handle = tokio::spawn(async move {
        if let Err(e) = desktop_stream_loop(&ws_url_owned, stop_clone).await {
            error!("desktop session ended with error: {e}");
        }
        info!("desktop session task exited");
    });

    Ok(DesktopSession {
        stop_signal: stop,
        handle,
    })
}

async fn desktop_stream_loop(ws_url: &str, stop: Arc<Notify>) -> Result<(), String> {
    let (ws_stream, _) = tokio_tungstenite::connect_async(ws_url)
        .await
        .map_err(|e| format!("WebSocket connect failed: {e}"))?;

    info!("connected to desktop relay WebSocket");

    let (mut ws_tx, mut ws_rx) = ws_stream.split();

    let fps = 10u64;
    let frame_interval = std::time::Duration::from_millis(1000 / fps);

    let stop_capture = stop.clone();
    let (frame_tx, mut frame_rx) = tokio::sync::mpsc::channel::<Vec<u8>>(4);
    let (meta_tx, meta_rx) = tokio::sync::oneshot::channel::<(u32, u32)>();

    let capture_task = tokio::task::spawn_blocking(move || {
        let display = match scrap::Display::primary() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("failed to get primary display: {e}");
                let _ = meta_tx.send((0, 0));
                return;
            }
        };
        let width = display.width() as u32;
        let height = display.height() as u32;

        let mut capturer = match scrap::Capturer::new(display) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to create capturer: {e}");
                let _ = meta_tx.send((0, 0));
                return;
            }
        };

        let _ = meta_tx.send((width, height));

        loop {
            if Arc::strong_count(&stop_capture) <= 1 {
                break;
            }

            match capturer.frame() {
                Ok(frame) => {
                    if let Some(jpeg) = encode_frame_jpeg(&frame, width, height) {
                        if frame_tx.blocking_send(jpeg).is_err() {
                            break;
                        }
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    tracing::error!("capture error: {e}");
                    break;
                }
            }
            std::thread::sleep(frame_interval);
        }
    });

    let (width, height) = meta_rx.await.map_err(|_| "capture thread died")?;
    if width == 0 {
        return Err("failed to initialize screen capture".into());
    }

    info!(width, height, "screen capture initialized");

    let meta = serde_json::json!({
        "type": "meta",
        "width": width,
        "height": height,
    });
    ws_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(
            meta.to_string().into(),
        ))
        .await
        .map_err(|e| format!("failed to send meta: {e}"))?;

    let send_task = tokio::spawn(async move {
        while let Some(jpeg) = frame_rx.recv().await {
            if ws_tx
                .send(tokio_tungstenite::tungstenite::Message::Binary(jpeg.into()))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let input_task = tokio::spawn(async move {
        let mut enigo = match enigo::Enigo::new(&enigo::Settings::default()) {
            Ok(e) => e,
            Err(e) => {
                error!("failed to create enigo: {e:?}");
                return;
            }
        };

        while let Some(Ok(msg)) = ws_rx.next().await {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                if let Ok(evt) = serde_json::from_str::<InputEvent>(&text) {
                    handle_input_event(&mut enigo, &evt);
                }
            }
        }
    });

    tokio::select! {
        _ = stop.notified() => {
            info!("desktop session stop signal received");
        }
        _ = send_task => {
            info!("send task ended");
        }
        _ = input_task => {
            info!("input task ended");
        }
    }

    capture_task.abort();
    Ok(())
}

fn encode_frame_jpeg(bgra: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    let expected = (width * height * 4) as usize;
    if bgra.len() < expected {
        return None;
    }

    let mut rgb = Vec::with_capacity((width * height * 3) as usize);
    for pixel in bgra[..expected].chunks_exact(4) {
        rgb.push(pixel[2]); // R
        rgb.push(pixel[1]); // G
        rgb.push(pixel[0]); // B
    }

    let img =
        image::RgbImage::from_raw(width, height, rgb)?;
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut buf, image::ImageFormat::Jpeg)
        .ok()?;
    Some(buf.into_inner())
}

fn handle_input_event(enigo: &mut enigo::Enigo, evt: &InputEvent) {
    use enigo::{Coordinate, Direction, Mouse, Keyboard, Button, Key};

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
            if let Some(key) = map_key(&evt.key, &evt.code) {
                let _ = enigo.key(key, Direction::Press);
            }
        }
        "keyup" => {
            if let Some(key) = map_key(&evt.key, &evt.code) {
                let _ = enigo.key(key, Direction::Release);
            }
        }
        "scroll" => {
            let _ = enigo.scroll(evt.delta_y as i32, enigo::Axis::Vertical);
        }
        _ => {}
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
            if ch.is_ascii() {
                Some(Key::Unicode(ch))
            } else {
                Some(Key::Unicode(ch))
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Main
// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "dmsx_agent=info".into()),
        )
        .init();

    let cfg = AgentConfig::from_env();
    info!(
        api = %cfg.api_base,
        tenant = %cfg.tenant_id,
        heartbeat = ?cfg.heartbeat_interval,
        poll = ?cfg.command_poll_interval,
        platform = detect_platform(),
        hostname = %System::host_name().unwrap_or_default(),
        "DMSX Agent starting"
    );

    // --- Configure RustDesk relay if specified ---
    if let Some(relay) = &cfg.rustdesk_relay {
        info!(relay = %relay, "configuring RustDesk to use self-hosted server");
        configure_rustdesk_server(relay);
    }
    let rd_info = detect_rustdesk();
    if let Some(rd_id) = rd_info.get("id").and_then(|v| v.as_str()) {
        info!(rustdesk_id = %rd_id, "RustDesk detected");
    } else {
        warn!("RustDesk not detected — remote desktop will not be available. Install from https://rustdesk.com");
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build HTTP client");

    // --- Register or find existing device ---
    let device_id = loop {
        match find_or_register_device(&client, &cfg).await {
            Ok(id) => break id,
            Err(e) => {
                error!("registration failed: {e}, retrying in 5s...");
                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            }
        }
    };

    info!(device_id = %device_id, "agent running — entering main loop");

    let mut heartbeat_tick =
        tokio::time::interval(cfg.heartbeat_interval);
    let mut command_tick =
        tokio::time::interval(cfg.command_poll_interval);

    let mut desktop_session: Option<DesktopSession> = None;

    if let Err(e) = heartbeat(&client, &cfg, &device_id).await {
        warn!("initial heartbeat failed: {e}");
    }

    loop {
        tokio::select! {
            _ = heartbeat_tick.tick() => {
                if let Err(e) = heartbeat(&client, &cfg, &device_id).await {
                    warn!("heartbeat error: {e}");
                }
            }
            _ = command_tick.tick() => {
                if let Err(e) = poll_and_execute(&client, &cfg, &device_id, &mut desktop_session).await {
                    warn!("command poll error: {e}");
                }
            }
            _ = tokio::signal::ctrl_c() => {
                info!("shutting down agent...");
                if let Some(session) = desktop_session.take() {
                    session.stop_signal.notify_one();
                    let _ = session.handle.await;
                }
                let url = cfg.tenant_url(&format!("/devices/{device_id}"));
                let _ = client.patch(&url)
                    .json(&serde_json::json!({"online_state": "offline"}))
                    .send()
                    .await;
                info!("goodbye");
                break;
            }
        }
    }
}
