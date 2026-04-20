use std::process::Stdio;

use reqwest::Client;
use tokio::process::Command as TokioCommand;
use tracing::{error, info, warn};
use dmsx_agent::api::{CommandItem, ListResponse, SubmitResultReq, UpdateStatusReq};
use dmsx_agent::config::AgentConfig;
use dmsx_agent::script::run_script;

use crate::desktop::{start_desktop_session, DesktopSession};

pub(crate) async fn poll_and_execute(
    client: &Client,
    cfg: &AgentConfig,
    device_id: &str,
    desktop_session: &mut Option<DesktopSession>,
) -> Result<(), Box<dyn std::error::Error>> {
    let url = cfg.tenant_url(&format!("/devices/{device_id}/commands?limit=10"));
    let resp: ListResponse<CommandItem> = client.get(&url).send().await?.json().await?;

    let mut queued: Vec<&CommandItem> = resp.items.iter().filter(|c| c.status == "queued").collect();

    if queued.is_empty() {
        return Ok(());
    }

    // API returns newest-first; execute oldest-first so compensating commands like
    // stop_desktop can naturally run after the start_desktop they are intended to cancel.
    queued.reverse();

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
    let action = cmd
        .payload
        .get("action")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let params = cmd
        .payload
        .get("params")
        .cloned()
        .unwrap_or(serde_json::json!({}));

    info!(command_id = %cmd_id, action = %action, "executing command");

    let status_url = cfg.tenant_url(&format!("/commands/{cmd_id}/status"));
    let _ = client
        .patch(&status_url)
        .json(&UpdateStatusReq {
            status: "running".into(),
        })
        .send()
        .await;

    let (exit_code, stdout, stderr) = match action {
        "start_desktop" => {
            if desktop_session.is_some() {
                info!("stopping existing desktop session before starting new one");
                if let Some(session) = desktop_session.take() {
                    session.stop().await;
                }
            }
            match start_desktop_session(&params).await {
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
            let target_session_id = params
                .get("session_id")
                .and_then(|v: &serde_json::Value| v.as_str())
                .unwrap_or_default();
            if desktop_session
                .as_ref()
                .map(|session| session.session_id == target_session_id)
                .unwrap_or(false)
            {
                if let Some(session) = desktop_session.take() {
                    session.stop().await;
                    (0, "desktop session stopped".into(), String::new())
                } else {
                    (0, "no active desktop session".into(), String::new())
                }
            } else {
                (
                    0,
                    format!("desktop session {target_session_id} already inactive"),
                    String::new(),
                )
            }
        }
        "run_script" => run_script(&params).await,
        "reboot" => {
            info!("reboot requested — scheduling system reboot");
            schedule_reboot().await
        }
        "shutdown" => {
            let delay = params
                .get("delay_seconds")
                .and_then(|v: &serde_json::Value| v.as_u64())
                .unwrap_or(30);
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
        "smoke_noop" => {
            info!("smoke_noop — no side effects");
            (0, "smoke_noop ok".into(), String::new())
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
        (
            "shutdown.exe",
            vec!["/s", "/t", &delay_str, "/c", "DMSX remote shutdown"],
        )
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
            &[
                "-e",
                r#"tell application "System Events" to keystroke "q" using {control down, command down}"#,
            ],
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
                        &[
                            "-NoProfile",
                            "-Command",
                            "Get-EventLog -LogName System -Newest 50 | Format-Table -AutoSize",
                        ],
                    )
                    .await
                }
                "agent" => (
                    0,
                    "(agent log collection not implemented yet)".into(),
                    String::new(),
                ),
                _ => (0, format!("(unknown log type: {lt})"), String::new()),
            }
        } else {
            match *lt {
                "system" => run_system_command("journalctl", &["-n", "100", "--no-pager"]).await,
                "agent" => (
                    0,
                    "(agent log collection not implemented yet)".into(),
                    String::new(),
                ),
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
