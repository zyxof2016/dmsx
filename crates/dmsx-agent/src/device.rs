use chrono::Utc;
use reqwest::Client;
use tracing::{info, warn};

use crate::api::{ClaimDeviceEnrollmentReq, CreateDeviceReq, Device, ListResponse};
use crate::config::AgentConfig;
use crate::platform::{detect_platform, hostname, os_version};
use crate::telemetry::collect_telemetry;

pub async fn find_or_register_device(
    client: &Client,
    cfg: &AgentConfig,
) -> Result<String, Box<dyn std::error::Error>> {
    let hostname = hostname();
    let platform = detect_platform();

    if let Some(enrollment_token) = &cfg.enrollment_token {
        let claim_url = format!(
            "{}/v1/tenants/{}/devices/claim-with-enrollment-token",
            cfg.api_base, cfg.tenant_id
        );
        let claim_body = ClaimDeviceEnrollmentReq {
            enrollment_token: enrollment_token.clone(),
            hostname: Some(hostname.clone()),
            os_version: os_version(),
            agent_version: Some(env!("CARGO_PKG_VERSION").into()),
            labels: serde_json::json!({
                "agent": "dmsx-agent",
                "claimed_at": Utc::now().to_rfc3339(),
            }),
        };
        let resp = client.post(&claim_url).json(&claim_body).send().await?;
        if resp.status().is_success() {
            let dev: Device = resp.json().await?;
            info!(device_id = %dev.id, registration_code = %dev.registration_code, "claimed device with enrollment token");
            return Ok(dev.id);
        }
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(format!("claim with enrollment token failed: {status} — {text}").into());
    }

    let search = cfg
        .registration_code
        .as_deref()
        .unwrap_or(hostname.as_str());

    let url = cfg.tenant_url(&format!(
        "/devices?search={search}&platform={platform}&limit=5"
    ));
    let resp: ListResponse<Device> = client.get(&url).send().await?.json().await?;

    if let Some(existing) = resp.items.into_iter().find(|device| {
        cfg.registration_code
            .as_deref()
            .map(|code| device.registration_code == code)
            .unwrap_or_else(|| device.hostname.as_deref() == Some(hostname.as_str()))
    }) {
        info!(device_id = %existing.id, "found existing device registration");
        return Ok(existing.id);
    }

    info!("no existing device found, registering new device...");
    let body = CreateDeviceReq {
        platform: platform.into(),
        registration_code: cfg.registration_code.clone(),
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

pub async fn heartbeat(
    client: &Client,
    cfg: &AgentConfig,
    device_id: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let telemetry = collect_telemetry();

    let device_url = cfg.tenant_url(&format!("/devices/{device_id}"));
    let _ = cfg
        .apply_device_auth(client.patch(&device_url))
        .json(&serde_json::json!({
            "online_state": "online",
            "agent_version": env!("CARGO_PKG_VERSION"),
            "os_version": os_version(),
        }))
        .send()
        .await;

    let shadow_reported_url = cfg.tenant_url(&format!("/devices/{device_id}/shadow/reported"));
    let resp = cfg
        .apply_device_auth(client.patch(&shadow_reported_url))
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

pub async fn mark_offline(client: &Client, cfg: &AgentConfig, device_id: &str) {
    let url = cfg.tenant_url(&format!("/devices/{device_id}"));
    let _ = cfg
        .apply_device_auth(client.patch(&url))
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
            registration_code: None,
            enrollment_token: None,
            heartbeat_interval: std::time::Duration::from_secs(30),
            command_poll_interval: std::time::Duration::from_secs(10),
            command_execution_timeout: std::time::Duration::from_secs(300),
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
                    { "id": "dev-existing", "registration_code": "DEV-EXISTING01", "hostname": current_hostname }
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
                "registration_code": "DEV-NEW000001",
                "hostname": current_hostname
            })))
            .expect(1)
            .mount(&server)
            .await;

        let device_id = find_or_register_device(&client, &cfg).await.unwrap();
        assert_eq!(device_id, "dev-new");
    }

    #[tokio::test]
    async fn find_or_register_device_prefers_registration_code_when_present() {
        let server = MockServer::start().await;
        let mut cfg = test_cfg(&server);
        cfg.registration_code = Some("DEV-BIND-0001".into());
        let client = test_client();

        Mock::given(method("GET"))
            .and(path("/v1/tenants/test-tenant/devices"))
            .and(query_param("search", "DEV-BIND-0001"))
            .and(query_param("platform", detect_platform()))
            .and(query_param("limit", "5"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "items": [
                    { "id": "dev-bound", "registration_code": "DEV-BIND-0001", "hostname": "different-host" }
                ]
            })))
            .expect(1)
            .mount(&server)
            .await;

        let device_id = find_or_register_device(&client, &cfg).await.unwrap();
        assert_eq!(device_id, "dev-bound");
    }

    #[tokio::test]
    async fn find_or_register_device_claims_with_enrollment_token_when_present() {
        let server = MockServer::start().await;
        let mut cfg = test_cfg(&server);
        cfg.enrollment_token = Some("v1.test.payload".into());
        let client = test_client();

        Mock::given(method("POST"))
            .and(path(
                "/v1/tenants/test-tenant/devices/claim-with-enrollment-token",
            ))
            .respond_with(ResponseTemplate::new(201).set_body_json(serde_json::json!({
                "id": "dev-claimed",
                "registration_code": "DEV-CLAIM-0001",
                "hostname": hostname(),
            })))
            .expect(1)
            .mount(&server)
            .await;

        let device_id = find_or_register_device(&client, &cfg).await.unwrap();
        assert_eq!(device_id, "dev-claimed");
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
