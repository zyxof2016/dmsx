#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub api_base: String,
    pub tenant_id: String,
    pub heartbeat_interval: std::time::Duration,
    pub command_poll_interval: std::time::Duration,
    pub rustdesk_relay: Option<String>,
}

impl AgentConfig {
    pub fn from_env() -> Self {
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

    pub fn tenant_url(&self, path: &str) -> String {
        format!("{}/v1/tenants/{}{}", self.api_base, self.tenant_id, path)
    }
}
