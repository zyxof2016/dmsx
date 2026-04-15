use chrono::Utc;
use reqwest::Client;
use tracing::{info, warn};

use crate::api::{CreateDeviceReq, Device, ListResponse};
use crate::config::AgentConfig;
use crate::platform::{detect_platform, hostname, os_version};
use crate::telemetry::collect_telemetry;

pub(crate) async fn find_or_register_device(
    client: &Client,
    cfg: &AgentConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let hostname = hostname();
    let platform = detect_platform();

    let url = cfg.tenant_url(&format!(
        "/devices?search={hostname}&platform={platform}&limit=5"
    ));
    let resp: ListResponse<Device> = client.get(&url).send().await?.json().await?;

    if let Some(existing) = resp
        .items
        .into_iter()
        .find(|device| device.hostname.as_deref() == Some(hostname.as_str()))
    {
        info!(device_id = %existing.id, "found existing device registration");
        return Ok(existing.id);
    }

    info!("no existing device found, registering new device...");
    let body = CreateDeviceReq {
        platform: platform.into(),
        hostname: Some(hostname.clone()),
        os_version: os_version(),
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

pub(crate) async fn heartbeat(
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
            "os_version": os_version(),
        }))
        .send()
        .await;

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
        Ok(_) => {
            info!("heartbeat sent — device state updated (shadow reported endpoint not available)");
        }
        Err(e) => warn!("heartbeat shadow update failed: {e}"),
    }

    Ok(())
}

pub(crate) async fn mark_offline(client: &Client, cfg: &AgentConfig, device_id: &str) {
    let url = cfg.tenant_url(&format!("/devices/{device_id}"));
    let _ = client
        .patch(&url)
        .json(&serde_json::json!({"online_state": "offline"}))
        .send()
        .await;
}

#[cfg(test)]
mod tests {
    use super::{find_or_register_device, heartbeat};
    use crate::config::AgentConfig;
    use crate::platform::{detect_platform, hostname};
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn test_cfg(server: &MockServer) -> AgentConfig {
        AgentConfig {
            api_base: server.uri(),
            tenant_id: "test-tenant".into(),
            heartbeat_interval: std::time::Duration::from_secs(30),
            command_poll_interval: std::time::Duration::from_secs(10),
            rustdesk_relay: None,
        }
    }

    fn test_client() -> reqwest::Client {
        reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("build test client")
    }

    #[tokio::test]
    async fn find_or_register_device_reuses_existing_device() {
        let server = MockServer::start().await;
        let cfg = test_cfg(&server);
        let client = test_client();
        let current_hostname = hostname();

        Mock::given(method("GET"))
            .and(path("/v1/tenants/test-tenant/devices"))
            .and(query_param("search", current_hostname.as_str()))
            .and(query_param("platform", detect_platform()))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    { "id": "dev-existing", "hostname": current_hostname }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let device_id = find_or_register_device(&client, &cfg).await.unwrap();
        assert_eq!(device_id, "dev-existing");
    }

    #[tokio::test]
    async fn find_or_register_device_registers_when_missing() {
        let server = MockServer::start().await;
        let cfg = test_cfg(&server);
        let client = test_client();
        let current_hostname = hostname();

        Mock::given(method("GET"))
            .and(path("/v1/tenants/test-tenant/devices"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": []
            })))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("POST"))
            .and(path("/v1/tenants/test-tenant/devices"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "id": "dev-new",
                "hostname": current_hostname
            })))
            .expect(1)
            .mount(&server)
            .await;

        let device_id = find_or_register_device(&client, &cfg).await.unwrap();
        assert_eq!(device_id, "dev-new");
    }

    #[tokio::test]
    async fn heartbeat_updates_device_and_shadow() {
        let server = MockServer::start().await;
        let cfg = test_cfg(&server);
        let client = test_client();
        let device_id = "dev-heartbeat";

        Mock::given(method("PATCH"))
            .and(path("/v1/tenants/test-tenant/devices/dev-heartbeat"))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        Mock::given(method("PATCH"))
            .and(path(
                "/v1/tenants/test-tenant/devices/dev-heartbeat/shadow/reported",
            ))
            .respond_with(ResponseTemplate::new(200))
            .expect(1)
            .mount(&server)
            .await;

        heartbeat(&client, &cfg, device_id).await.unwrap();
    }
}
