use std::path::{Path, PathBuf};

use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub api_base: String,
    pub tenant_id: String,
    pub registration_code: Option<String>,
    pub enrollment_token: Option<String>,
    pub heartbeat_interval: std::time::Duration,
    pub command_poll_interval: std::time::Duration,
    pub command_execution_timeout: std::time::Duration,
    pub rustdesk_relay: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct AgentConfigFile {
    api_base: Option<String>,
    tenant_id: Option<String>,
    registration_code: Option<String>,
    enrollment_token: Option<String>,
    heartbeat_secs: Option<u64>,
    poll_secs: Option<u64>,
    command_execution_timeout_secs: Option<u64>,
    rustdesk_relay: Option<String>,
}

impl AgentConfig {
    pub fn from_env() -> Self {
        Self::from_sources(None)
    }

    pub fn from_sources(config_path: Option<&Path>) -> Self {
        let file = load_config_file(config_path).unwrap_or_default();
        Self {
            api_base: env_string("DMSX_API_URL")
                .or(file.api_base)
                .unwrap_or_else(|| "http://127.0.0.1:8080".into()),
            tenant_id: env_string("DMSX_TENANT_ID")
                .or(file.tenant_id)
                .unwrap_or_else(|| "00000000-0000-0000-0000-000000000001".into()),
            registration_code: env_string("DMSX_DEVICE_REGISTRATION_CODE")
                .or(file.registration_code)
                .map(|value| value.trim().to_ascii_uppercase())
                .filter(|value| !value.is_empty()),
            enrollment_token: env_string("DMSX_DEVICE_ENROLLMENT_TOKEN")
                .or(file.enrollment_token)
                .map(|value| value.trim().to_string())
                .filter(|value| !value.is_empty()),
            heartbeat_interval: std::time::Duration::from_secs(
                env_u64("DMSX_HEARTBEAT_SECS")
                    .or(file.heartbeat_secs)
                    .unwrap_or(30),
            ),
            command_poll_interval: std::time::Duration::from_secs(
                env_u64("DMSX_POLL_SECS").or(file.poll_secs).unwrap_or(10),
            ),
            command_execution_timeout: std::time::Duration::from_secs(
                env_u64("DMSX_COMMAND_EXEC_TIMEOUT_SECS")
                    .or(file.command_execution_timeout_secs)
                    .unwrap_or(300),
            ),
            rustdesk_relay: env_string("DMSX_RUSTDESK_RELAY").or(file.rustdesk_relay),
        }
    }

    pub fn tenant_url(&self, path: &str) -> String {
        format!("{}/v1/tenants/{}{}", self.api_base, self.tenant_id, path)
    }

    pub fn apply_device_auth(&self, request: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
        match &self.enrollment_token {
            Some(token) => request.header("x-dmsx-device-token", token),
            None => request,
        }
    }
}

fn load_config_file(config_path: Option<&Path>) -> Option<AgentConfigFile> {
    let path = config_path
        .map(Path::to_path_buf)
        .or_else(|| env_string("DMSX_AGENT_CONFIG").map(PathBuf::from))?;
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}

fn env_string(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn env_u64(name: &str) -> Option<u64> {
    env_string(name).and_then(|value| value.parse().ok())
}
