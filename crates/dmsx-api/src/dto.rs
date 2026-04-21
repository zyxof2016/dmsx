use chrono::{DateTime, Utc};
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

#[derive(Debug, Serialize)]
pub struct CountBucket {
    pub label: String,
    pub count: i64,
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
    pub registration_code: Option<String>,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub site_id: Option<Uuid>,
    pub primary_group_id: Option<Uuid>,
    #[serde(default = "default_json_obj")]
    pub labels: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct BatchCreateDevicesReq {
    pub items: Vec<CreateDeviceReq>,
    pub issue_enrollment_tokens: Option<bool>,
    pub ttl_seconds: Option<i64>,
}

impl BatchCreateDevicesReq {
    pub fn issue_enrollment_tokens(&self) -> bool {
        self.issue_enrollment_tokens.unwrap_or(false)
    }

    pub fn ttl_seconds(&self) -> i64 {
        self.ttl_seconds.unwrap_or(1800)
    }

    pub fn validate(&self) -> Result<(), DmsxError> {
        if self.items.is_empty() || self.items.len() > 200 {
            return Err(DmsxError::Validation(
                "items must contain between 1 and 200 devices".into(),
            ));
        }
        for item in &self.items {
            item.validate()?;
        }
        if self.issue_enrollment_tokens() && !(60..=86400).contains(&self.ttl_seconds()) {
            return Err(DmsxError::Validation(
                "ttl_seconds must be between 60 and 86400".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Serialize)]
pub struct BatchCreateDevicesResponse {
    pub devices: Vec<Device>,
    pub enrollment_tokens: Vec<DeviceEnrollmentToken>,
}

impl CreateDeviceReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(h) = &self.hostname {
            check_len("hostname", h, 1, 253)?;
        }
        if let Some(code) = &self.registration_code {
            check_len("registration_code", code, 4, 64)?;
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
    pub registration_code: Option<String>,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub enroll_status: Option<EnrollStatus>,
    pub online_state: Option<OnlineState>,
    pub labels: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct DeviceEnrollmentToken {
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub registration_code: String,
    pub device_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct IssueDeviceEnrollmentTokenReq {
    pub ttl_seconds: Option<i64>,
}

impl IssueDeviceEnrollmentTokenReq {
    pub fn ttl_seconds(&self) -> i64 {
        self.ttl_seconds.unwrap_or(1800)
    }

    pub fn validate(&self) -> Result<(), DmsxError> {
        let ttl = self.ttl_seconds();
        if !(60..=86400).contains(&ttl) {
            return Err(DmsxError::Validation(
                "ttl_seconds must be between 60 and 86400".into(),
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct ClaimDeviceEnrollmentReq {
    pub enrollment_token: String,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    #[serde(default = "default_json_obj")]
    pub labels: serde_json::Value,
}

impl ClaimDeviceEnrollmentReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("enrollment_token", &self.enrollment_token, 10, 4000)?;
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

impl UpdateDeviceReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(code) = &self.registration_code {
            check_len("registration_code", code, 4, 64)?;
        }
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
// Tenant hierarchy (orgs / sites / groups)
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateTenantReq {
    pub name: String,
}

impl CreateTenantReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateOrgReq {
    pub name: String,
}

impl CreateOrgReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateSiteReq {
    pub name: String,
}

impl CreateSiteReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
        Ok(())
    }
}

#[derive(Debug, Deserialize)]
pub struct CreateGroupReq {
    pub name: String,
}

impl CreateGroupReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        check_len("name", &self.name, 1, 200)?;
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

#[derive(Debug, Deserialize)]
pub struct IssueEvidenceUploadTokenReq {
    pub content_type: Option<String>,
    pub expires_in_seconds: Option<i64>,
}

impl IssueEvidenceUploadTokenReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        if let Some(content_type) = &self.content_type {
            check_len("content_type", content_type, 1, 255)?;
        }
        if let Some(v) = self.expires_in_seconds {
            if !(60..=3600).contains(&v) {
                return Err(DmsxError::Validation(
                    "expires_in_seconds must be between 60 and 3600".into(),
                ));
            }
        }
        Ok(())
    }

    pub fn expires_in_seconds(&self) -> i64 {
        self.expires_in_seconds.unwrap_or(900)
    }
}

#[derive(Debug, Serialize)]
pub struct EvidenceUploadToken {
    pub upload_token: String,
    pub tenant_id: Uuid,
    pub device_id: Uuid,
    pub command_id: Uuid,
    pub content_type: Option<String>,
    pub expires_at: DateTime<Utc>,
}

// ---------------------------------------------------------------------------
// helpers
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Admin / Observability / Config
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct AuditLog {
    pub id: Uuid,
    pub created_at: DateTime<Utc>,
    pub actor_user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
}

impl AuditLogListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }
    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }
}

#[derive(Debug, Deserialize)]
pub struct SystemSettingUpsertReq {
    pub value: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct SystemSetting {
    pub key: String,
    pub value: serde_json::Value,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct RbacRole {
    pub name: String,
    pub scope: String,
    pub description: String,
    pub platform_read: bool,
    pub platform_write: bool,
    pub tenant_read: bool,
    pub tenant_write: bool,
}

#[derive(Debug, Serialize)]
pub struct PlatformTenantSummary {
    pub id: uuid::Uuid,
    pub name: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub device_count: i64,
    pub policy_count: i64,
    pub command_count: i64,
}

#[derive(Debug, Deserialize)]
pub struct PlatformTenantListParams {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub search: Option<String>,
}

impl PlatformTenantListParams {
    pub fn limit(&self) -> i64 {
        clamp_limit(self.limit)
    }

    pub fn offset(&self) -> i64 {
        clamp_offset(self.offset)
    }

    pub fn search_term(&self) -> Option<&str> {
        self.search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }
}

#[derive(Debug, Serialize)]
pub struct PlatformQuota {
    pub key: String,
    pub limit: i64,
    pub used: i64,
    pub unit: String,
}

#[derive(Debug, Serialize)]
pub struct PlatformHealth {
    pub status: String,
    pub tenant_count: i64,
    pub device_count: i64,
    pub policy_count: i64,
    pub command_count: i64,
    pub artifact_count: i64,
    pub audit_log_count: i64,
    pub livekit_enabled: bool,
    pub redis_enabled: bool,
    pub command_bus_enabled: bool,
}

#[derive(Debug, Deserialize)]
pub struct PolicyEditorPublishReq {
    pub name: String,
    pub description: Option<String>,
    pub scope_kind: PolicyScopeKind,
    pub scope_expr: String,
}

impl PolicyEditorPublishReq {
    pub fn validate(&self) -> Result<(), DmsxError> {
        // Keep validation consistent with CreatePolicyReq for name/description.
        check_len("name", &self.name, 1, 200)?;
        if let Some(d) = &self.description {
            check_len("description", d, 0, 2000)?;
        }
        check_len("scope_expr", &self.scope_expr, 1, 5000)?;
        Ok(())
    }
}

fn default_json_obj() -> serde_json::Value {
    serde_json::json!({})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_list_params_clamp_limit_and_offset() {
        let params = DeviceListParams {
            limit: Some(999),
            offset: Some(-5),
            search: None,
            platform: None,
            enroll_status: None,
            online_state: None,
        };

        assert_eq!(params.limit(), 200);
        assert_eq!(params.offset(), 0);
    }

    #[test]
    fn create_command_req_rejects_invalid_ttl() {
        let req = CreateCommandReq {
            target_device_id: Uuid::new_v4(),
            payload: serde_json::json!({}),
            priority: Some(0),
            ttl_seconds: Some(10),
            idempotency_key: None,
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }

    #[test]
    fn create_artifact_req_rejects_invalid_sha256() {
        let req = CreateArtifactReq {
            name: "agent".into(),
            version: "1.0.0".into(),
            sha256: "abc".into(),
            channel: None,
            object_key: "artifacts/agent".into(),
            metadata: None,
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }

    #[test]
    fn device_action_req_requires_script_for_run_script() {
        let req = DeviceActionReq {
            action: "run_script".into(),
            params: serde_json::json!({}),
            priority: None,
            ttl_seconds: None,
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }

    #[test]
    fn device_action_req_requires_confirm_for_wipe() {
        let req = DeviceActionReq {
            action: "wipe".into(),
            params: serde_json::json!({}),
            priority: None,
            ttl_seconds: None,
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }

    #[test]
    fn issue_evidence_upload_token_req_rejects_ttl_out_of_range() {
        let req = IssueEvidenceUploadTokenReq {
            content_type: Some("text/plain".into()),
            expires_in_seconds: Some(30),
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }

    #[test]
    fn issue_evidence_upload_token_req_defaults_ttl() {
        let req = IssueEvidenceUploadTokenReq {
            content_type: None,
            expires_in_seconds: None,
        };

        assert_eq!(req.expires_in_seconds(), 900);
    }

    #[test]
    fn update_shadow_desired_requires_object() {
        let req = UpdateShadowDesiredReq {
            desired: serde_json::json!(["not-object"]),
        };

        let err = req.validate().unwrap_err();
        assert!(matches!(err, DmsxError::Validation(_)));
    }
}
