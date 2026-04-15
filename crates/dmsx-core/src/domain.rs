//! 多租户资源层级：Tenant → Org → Site → Group → Device。
//! 策略、命令、制品、审计均带 `tenant_id` 以实现行级隔离。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// --- IDs (transparent newtypes over Uuid) ---

macro_rules! uuid_newtype {
    ($name:ident) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
        #[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
        #[cfg_attr(feature = "sqlx", sqlx(transparent))]
        #[serde(transparent)]
        pub struct $name(pub Uuid);
    };
}

uuid_newtype!(TenantId);
uuid_newtype!(OrgId);
uuid_newtype!(SiteId);
uuid_newtype!(GroupId);
uuid_newtype!(DeviceId);
uuid_newtype!(PolicyId);
uuid_newtype!(PolicyRevisionId);
uuid_newtype!(CommandId);
uuid_newtype!(ArtifactId);
uuid_newtype!(UserId);

// --- Enums ---

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "device_platform", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum DevicePlatform {
    Windows,
    Linux,
    Macos,
    Ios,
    Android,
    Edge,
    Other,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "enroll_status", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum EnrollStatus {
    Pending,
    Active,
    Revoked,
    Blocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "online_state", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum OnlineState {
    Unknown,
    Online,
    Offline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "command_status", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Queued,
    Delivered,
    Acked,
    Running,
    Succeeded,
    Failed,
    Expired,
    Cancelled,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "policy_scope_kind", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum PolicyScopeKind {
    Tenant,
    Org,
    Site,
    Group,
    Label,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "finding_severity", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::Type))]
#[cfg_attr(
    feature = "sqlx",
    sqlx(type_name = "finding_status", rename_all = "snake_case")
)]
#[serde(rename_all = "snake_case")]
pub enum FindingStatus {
    Open,
    Accepted,
    Fixed,
    FalsePositive,
}

// --- Entities (mirror DB, derive FromRow when sqlx is enabled) ---

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Tenant {
    pub id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Org {
    pub id: OrgId,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Site {
    pub id: SiteId,
    pub org_id: OrgId,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Group {
    pub id: GroupId,
    pub site_id: SiteId,
    pub tenant_id: TenantId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Device {
    pub id: DeviceId,
    pub tenant_id: TenantId,
    pub site_id: Option<SiteId>,
    pub primary_group_id: Option<GroupId>,
    pub platform: DevicePlatform,
    pub hostname: Option<String>,
    pub os_version: Option<String>,
    pub agent_version: Option<String>,
    pub enroll_status: EnrollStatus,
    pub online_state: OnlineState,
    pub last_seen_at: Option<DateTime<Utc>>,
    pub labels: serde_json::Value,
    pub capabilities: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Policy {
    pub id: PolicyId,
    pub tenant_id: TenantId,
    pub name: String,
    pub description: Option<String>,
    pub scope_kind: PolicyScopeKind,
    pub scope_org_id: Option<OrgId>,
    pub scope_site_id: Option<SiteId>,
    pub scope_group_id: Option<GroupId>,
    pub scope_expr: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct PolicyRevision {
    pub id: PolicyRevisionId,
    pub policy_id: PolicyId,
    pub tenant_id: TenantId,
    pub version: i32,
    pub spec: serde_json::Value,
    pub rollout: serde_json::Value,
    pub published_at: DateTime<Utc>,
    pub published_by: Option<UserId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Command {
    pub id: CommandId,
    pub tenant_id: TenantId,
    pub idempotency_key: Option<String>,
    pub target_device_id: DeviceId,
    pub payload: serde_json::Value,
    pub priority: i16,
    pub ttl_seconds: i32,
    pub status: CommandStatus,
    pub created_by: Option<UserId>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct Artifact {
    pub id: ArtifactId,
    pub tenant_id: TenantId,
    pub name: String,
    pub version: String,
    pub sha256: String,
    pub signature_b64: Option<String>,
    pub channel: String,
    pub object_key: String,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct AuditLog {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub actor_user_id: Option<UserId>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: String,
    pub payload: serde_json::Value,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct DeviceShadow {
    pub device_id: DeviceId,
    pub tenant_id: TenantId,
    pub reported: serde_json::Value,
    pub desired: serde_json::Value,
    pub reported_at: Option<DateTime<Utc>>,
    pub desired_at: Option<DateTime<Utc>>,
    pub version: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct CommandResult {
    pub command_id: CommandId,
    pub tenant_id: TenantId,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
    pub evidence_key: Option<String>,
    pub reported_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "sqlx", derive(sqlx::FromRow))]
pub struct ComplianceFinding {
    pub id: Uuid,
    pub tenant_id: TenantId,
    pub device_id: DeviceId,
    pub rule_id: String,
    pub title: String,
    pub severity: Severity,
    pub status: FindingStatus,
    pub evidence_object_key: Option<String>,
    pub details: serde_json::Value,
    pub detected_at: DateTime<Utc>,
}
