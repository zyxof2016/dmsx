// ---- Domain entities (mirror backend JSON) ----

export interface Device {
  id: string;
  tenant_id: string;
  registration_code: string;
  site_id: string | null;
  primary_group_id: string | null;
  platform:
    | "windows"
    | "linux"
    | "macos"
    | "ios"
    | "android"
    | "edge"
    | "other";
  hostname: string | null;
  os_version: string | null;
  agent_version: string | null;
  enroll_status: "pending" | "active" | "revoked" | "blocked";
  online_state: "unknown" | "online" | "offline";
  last_seen_at: string | null;
  labels: Record<string, unknown>;
  capabilities: Record<string, unknown>;
  created_at: string;
  updated_at: string;
}

export interface DeviceEnrollmentToken {
  token: string;
  expires_at: string;
  registration_code: string;
  device_id: string;
}

export interface IssueDeviceEnrollmentTokenReq {
  ttl_seconds?: number;
}

export interface BatchCreateDevicesReq {
  items: CreateDeviceReq[];
  issue_enrollment_tokens?: boolean;
  ttl_seconds?: number;
}

export interface BatchCreateDevicesResponse {
  batch_id: string;
  devices: Device[];
  enrollment_tokens: DeviceEnrollmentToken[];
}

export interface DeviceEnrollmentBatchResponse {
  batch_id: string;
  devices: Device[];
  enrollment_tokens: DeviceEnrollmentToken[];
  created_at: string;
}

export interface DeviceEnrollmentBatchSummary {
  batch_id: string;
  devices: Device[];
  enrollment_tokens: DeviceEnrollmentToken[];
  created_at: string;
}

export interface Policy {
  id: string;
  tenant_id: string;
  name: string;
  description: string | null;
  scope_kind: "tenant" | "org" | "site" | "group" | "label";
  scope_org_id: string | null;
  scope_site_id: string | null;
  scope_group_id: string | null;
  scope_expr: string | null;
  created_at: string;
  updated_at: string;
}

export interface Command {
  id: string;
  tenant_id: string;
  idempotency_key: string | null;
  target_device_id: string;
  payload: Record<string, unknown>;
  priority: number;
  ttl_seconds: number;
  status:
    | "queued"
    | "delivered"
    | "acked"
    | "running"
    | "succeeded"
    | "failed"
    | "expired"
    | "cancelled";
  created_by: string | null;
  created_at: string;
  updated_at: string;
}

export interface Artifact {
  id: string;
  tenant_id: string;
  name: string;
  version: string;
  sha256: string;
  signature_b64: string | null;
  channel: string;
  object_key: string;
  metadata: Record<string, unknown>;
  created_at: string;
}

export interface ComplianceFinding {
  id: string;
  tenant_id: string;
  device_id: string;
  rule_id: string;
  title: string;
  severity: "info" | "low" | "medium" | "high" | "critical";
  status: "open" | "accepted" | "fixed" | "false_positive";
  evidence_object_key: string | null;
  details: Record<string, unknown>;
  detected_at: string;
}

// ---- Dashboard ----

export interface CountBucket {
  label: string;
  count: number;
}

export interface DashboardStats {
  device_total: number;
  device_online: number;
  policy_count: number;
  command_pending: number;
  finding_open: number;
  platforms: CountBucket[];
  command_statuses: CountBucket[];
  finding_severities: CountBucket[];
}

// ---- Paginated list wrapper ----

export interface ListResponse<T> {
  items: T[];
  total: number;
  limit: number;
  offset: number;
}

// ---- Create request types ----

export interface CreateDeviceReq {
  platform: Device["platform"];
  registration_code?: string;
  hostname?: string;
  os_version?: string;
  agent_version?: string;
}

export interface CreatePolicyReq {
  name: string;
  description?: string;
  scope_kind: Policy["scope_kind"];
}

export interface CreateCommandReq {
  target_device_id: string;
  payload: Record<string, unknown>;
  priority?: number;
  ttl_seconds?: number;
}

export interface CreateArtifactReq {
  name: string;
  version: string;
  sha256: string;
  channel?: string;
  object_key: string;
}

// ---- Device Shadow ----

export interface ShadowResponse {
  device_id: string;
  reported: Record<string, unknown>;
  desired: Record<string, unknown>;
  delta: Record<string, unknown>;
  reported_at: string | null;
  desired_at: string | null;
  version: number;
}

export interface UpdateShadowDesiredReq {
  desired: Record<string, unknown>;
}

// ---- Device Actions (remote control) ----

export type DeviceActionType =
  | "reboot"
  | "lock_screen"
  | "shutdown"
  | "wipe"
  | "run_script"
  | "install_update"
  | "collect_logs";

export interface DeviceActionReq {
  action: DeviceActionType;
  params?: Record<string, unknown>;
  priority?: number;
  ttl_seconds?: number;
}

// ---- Command Result ----

export interface CommandResult {
  command_id: string;
  tenant_id: string;
  exit_code: number | null;
  stdout: string;
  stderr: string;
  evidence_key: string | null;
  reported_at: string;
}

export interface SubmitCommandResultReq {
  exit_code?: number;
  stdout?: string;
  stderr?: string;
  evidence_key?: string;
}

export interface UpdateCommandStatusReq {
  status: Command["status"];
}

// ---- Desktop Session ----

export interface DesktopSessionResponse {
  room: string;
  token: string;
  livekit_url: string;
  session_id: string;
}

export interface DesktopSessionCreateReq {
  width?: number;
  height?: number;
}

export interface LivekitConfigResponse {
  enabled: boolean;
  url: string;
}

// ---- Admin / Config / Auditing ----

export interface AuditLog {
  id: string;
  created_at: string;
  actor_user_id: string | null;
  action: string;
  resource_type: string;
  resource_id: string;
  payload: Record<string, unknown>;
}

export interface AuditLogListParams {
  limit?: number;
  offset?: number;
  action?: string;
  resource_type?: string;
}

export interface SystemSetting {
  key: string;
  value: Record<string, unknown>;
  updated_at: string;
}

export interface SystemSettingUpsertReq {
  value: Record<string, unknown>;
}

export interface RbacRole {
  name: string;
  scope: string;
  description: string;
  platform_read: boolean;
  platform_write: boolean;
  tenant_read: boolean;
  tenant_write: boolean;
}

export interface Tenant {
  id: string;
  name: string;
  created_at?: string;
  updated_at?: string;
}

export interface PlatformTenantSummary {
  id: string;
  name: string;
  created_at: string;
  device_count: number;
  policy_count: number;
  command_count: number;
}

export interface PlatformQuota {
  key: string;
  limit: number;
  used: number;
  unit: string;
}

export interface PlatformHealth {
  status: string;
  tenant_count: number;
  device_count: number;
  policy_count: number;
  command_count: number;
  artifact_count: number;
  audit_log_count: number;
  livekit_enabled: boolean;
  redis_enabled: boolean;
  command_bus_enabled: boolean;
}

export interface PolicyRevision {
  id: string;
  policy_id: string;
  tenant_id: string;
  version: number;
  spec: Record<string, unknown>;
  rollout: Record<string, unknown>;
  published_at: string;
  published_by: string | null;
}

export interface PolicyEditorPublishReq {
  name: string;
  description?: string | null;
  scope_kind: Policy["scope_kind"];
  scope_expr: string;
}

// ---- Query params for list endpoints ----

export type ListParams = Record<string, string | number | undefined>;
