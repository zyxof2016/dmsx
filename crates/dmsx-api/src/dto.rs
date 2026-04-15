use dmsx_core::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ---------------------------------------------------------------------------
// Pagination helpers
// ---------------------------------------------------------------------------

fn clamp_limit(v: Option<i64>) -> i64 {
    v.unwrap_or(50).clamp(1, 200)
}

fn clamp_offset(v: Option<i64>) -> i64 {
    v.unwrap_or(0).max(0)
}

fn check_len(field: &str, val: &str, min: usize, max: usize) -> Result<(), DmsxError> {
    let len = val.trim().len();
    if len < min || len > max {
        return Err(DmsxError::Validation(format!(
            "{field} must be {min}-{max} characters, got {len}"
        )));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// List response envelope (with pagination metadata)
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ListResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

// ---------------------------------------------------------------------------
// Dashboard
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct DashboardStats {
    pub device_total: i64,
    pub device_online: i64,
    pub policy_count: i64,
    pub command_pending: i64,
    pub finding_open: i64,
    pub platforms: Vec<CountBucket>,
    pub command_statuses: Vec<CountBucket>,
    pub finding_severities: Vec<CountBucket>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct CountBucket {
    pub label: String,
    pub count: i64,
}

#[derive(Debug, sqlx::FromRow)]
pub struct StatsRow {
    pub device_total: i64,
    pub device_online: i64,
    pub policy_count: i64,
    pub command_pending: i64,
    pub finding_open: i64,
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeviceListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub platform: Option<DevicePlatform>,
    pub enroll_status: Option<EnrollStatus>,
    pub online_state: Option<OnlineState>,
}

impl DeviceListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
    pub fn search_term(&self) -> Option<&str> {
        self.search.as_deref().filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateDeviceReq {
    pub platform: DevicePlatform,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub site_id: Option<Uuid>,
    pub primary_group_id: Option<Uuid>,
    #[serde(default = "default_json_obj")]
    pub labels: serde_json::Value,
}

impl CreateDeviceReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(h) = &self.hostname {
            check_len("hostname", h, 1, 253)?;
        }
        if let Some(v) = &self.os_version {
            check_len("os_version", v, 1, 200)?;
        }
        if let Some(v) = &self.agent_version {
            check_len("agent_version", v, 1, 100)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceReq {
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub enroll_status: Option<EnrollStatus>,
    pub online_state: Option<OnlineState>,
    pub labels: Option<serde_json::Value>,
}

impl UpdateDeviceReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(h) = &self.hostname {
            check_len("hostname", h, 1, 253)?;
        }
        if let Some(v) = &self.os_version {
            check_len("os_version", v, 1, 200)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct PolicyListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub scope_kind: Option<PolicyScopeKind>,
}

impl PolicyListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
    pub fn search_term(&self) -> Option<&str> {
        self.search.as_deref().filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePolicyReq {
    pub name: String,
    pub description: Option<String>,
    pub scope_kind: PolicyScopeKind,
}

impl CreatePolicyReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
        if let Some(d) = &self.description {
            check_len("description", d, 0, 2000)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdatePolicyReq {
    pub name: Option<String>,
    pub description: Option<String>,
    pub scope_kind: Option<PolicyScopeKind>,
}

impl UpdatePolicyReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(n) = &self.name {
            check_len("name", n, 1, 200)?;
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct PublishPolicyReq {
    #[serde(default)]
    pub spec: serde_json::Value,
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CommandListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub status: Option<CommandStatus>,
    pub target_device_id: Option<Uuid>,
}

impl CommandListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateCommandReq {
    pub target_device_id: Uuid,
    pub payload: serde_json::Value,
    pub priority: Option<i16>,
    pub ttl_seconds: Option<i32>,
    pub idempotency_key: Option<String>,
}

impl CreateCommandReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        let pri = self.priority.unwrap_or(0);
        if !(-10..=10).contains(&pri) {
            return Err(DmsxError::Validation(
                "priority must be between -10 and 10".into(),
            ));
        }
        let ttl = self.ttl_seconds.unwrap_or(3600);
        if !(60..=86400).contains(&ttl) {
            return Err(DmsxError::Validation(
                "ttl_seconds must be between 60 and 86400".into(),
            ));
        }
        if let Some(k) = &self.idempotency_key {
            check_len("idempotency_key", k, 1, 200)?;
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ArtifactListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

impl ArtifactListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
    pub fn search_term(&self) -> Option<&str> {
        self.search.as_deref().filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateArtifactReq {
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub channel: Option<String>,
    pub object_key: String,
    pub metadata: Option<serde_json::Value>,
}

impl CreateArtifactReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
        check_len("version", &self.version, 1, 100)?;
        check_len("object_key", &self.object_key, 1, 500)?;
        if self.sha256.len() != 64 || !self.sha256.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(DmsxError::Validation(
                "sha256 must be a 64-character hex string".into(),
            ));
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Compliance
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct FindingListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
    pub severity: Option<Severity>,
    pub status: Option<FindingStatus>,
}

impl FindingListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
    pub fn search_term(&self) -> Option<&str> {
        self.search.as_deref().filter(|s| !s.is_empty())
    }
}

// ---------------------------------------------------------------------------
// Device Shadow
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct UpdateShadowDesiredReq {
    pub desired: serde_json::Value,
}

impl UpdateShadowDesiredReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if !self.desired.is_object() {
            return Err(DmsxError::Validation(
                "desired must be a JSON object".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct UpdateShadowReportedReq {
    pub reported: serde_json::Value,
}

impl UpdateShadowReportedReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if !self.reported.is_object() {
            return Err(DmsxError::Validation(
                "reported must be a JSON object".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct ShadowResponse {
    pub device_id: Uuid,
    pub reported: serde_json::Value,
    pub desired: serde_json::Value,
    pub delta: serde_json::Value,
    pub reported_at: Option<chrono::DateTime<chrono::Utc>>,
    pub desired_at: Option<chrono::DateTime<chrono::Utc>>,
    pub version: i64,
}

// ---------------------------------------------------------------------------
// Device Actions (remote control)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct DeviceActionReq {
    pub action: String,
    #[serde(default = "default_json_obj")]
    pub params: serde_json::Value,
    pub priority: Option<i16>,
    pub ttl_seconds: Option<i32>,
}

impl DeviceActionReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        const VALID_ACTIONS: &[&str] = &[
            "reboot",
            "lock_screen",
            "shutdown",
            "wipe",
            "run_script",
            "install_update",
            "collect_logs",
        ];
        if !VALID_ACTIONS.contains(&self.action.as_str()) {
            return Err(DmsxError::Validation(format!(
                "unknown action '{}', valid: {}",
                self.action,
                VALID_ACTIONS.join(", ")
            )));
        }
        if self.action == "run_script" {
            let script = self.params.get("script").and_then(|v| v.as_str());
            if script.map_or(true, |s| s.is_empty()) {
                return Err(DmsxError::Validation(
                    "run_script requires non-empty params.script".into(),
                ));
            }
        }
        if self.action == "wipe" {
            let confirm = self.params.get("confirm").and_then(|v| v.as_bool());
            if confirm != Some(true) {
                return Err(DmsxError::Validation(
                    "wipe requires params.confirm = true".into(),
                ));
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Command lifecycle
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct UpdateCommandStatusReq {
    pub status: CommandStatus,
}

#[derive(Debug, Deserialize)]
pub struct SubmitCommandResultReq {
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub stdout: String,
    #[serde(default)]
    pub stderr: String,
    pub evidence_key: Option<String>,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

fn default_json_obj() -> serde_json::Value {
    serde_json::json!({})
}
