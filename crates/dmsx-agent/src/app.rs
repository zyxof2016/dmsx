use reqwest::Client;
use tracing::{error, info, warn};
use dmsx_agent::config::AgentConfig;
use dmsx_agent::device::{find_or_register_device, heartbeat, mark_offline};
use dmsx_agent::platform::{detect_platform, hostname};
use dmsx_agent::rustdesk::{configure_rustdesk_server, detect_rustdesk};

use crate::command_runner::poll_and_execute;
use crate::desktop::DesktopSession;

pub(crate) async fn run() {
    let cfg = AgentConfig::from_env();
    info!(
        api = %cfg.api_base,
        tenant = %cfg.tenant_id,
        heartbeat = ?cfg.heartbeat_interval,
        poll = ?cfg.command_poll_interval,
        command_execution_timeout = ?cfg.command_execution_timeout,
        platform = detect_platform(),
        hostname = %hostname(),
        "DMSX Agent starting"
    );

    if let Some(relay) = &cfg.rustdesk_relay {
        info!(relay = %relay, "configuring RustDesk to use self-hosted server");
        configure_rustdesk_server(relay);
    }
    let rd_info = detect_rustdesk();
    if let Some(rd_id) = rd_info.get("id").and_then(|v| v.as_str()) {
        info!(rustdesk_id = %rd_id, "RustDesk detected");
    } else {
        warn!(
            "RustDesk not detected — remote desktop will not be available. Install from https://rustdesk.com"
        );
    }

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .expect("failed to build HTTP client");

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

    let mut heartbeat_tick = tokio::time::interval(cfg.heartbeat_interval);
    let mut command_tick = tokio::time::interval(cfg.command_poll_interval);
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
                    session.stop().await;
                }
                mark_offline(&client, &cfg, &device_id).await;
                info!("goodbye");
                break;
            }
        }
    }
}
