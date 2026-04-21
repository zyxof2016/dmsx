use axum::{
    extract::{Extension, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use dmsx_core::*;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::dto::*;
use crate::services::{
    artifacts, audit, commands, compliance, devices, hierarchy, platform, policies, shadow, stats,
    system_settings,
};
use crate::state::AppState;

pub type ApiResult<T> = Result<T, DmsxError>;

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "dmsx-api" }))
}

pub async fn ready(State(st): State<AppState>) -> impl IntoResponse {
    let auth = crate::auth::auth_readiness(&st.auth).await;
    let status = if auth.ready {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };
    (
        status,
        Json(json!({
            "status": if auth.ready { "ok" } else { "not_ready" },
            "service": "dmsx-api",
            "auth": auth,
        })),
    )
}

// ---------------------------------------------------------------------------
// Dashboard Stats
// ---------------------------------------------------------------------------

pub async fn stats(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<DashboardStats>> {
    Ok(Json(stats::get_stats(&st, &ctx, tenant_id).await?))
}

// ---------------------------------------------------------------------------
// Tenant hierarchy
// ---------------------------------------------------------------------------

pub async fn tenants_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Json(body): Json<CreateTenantReq>,
) -> ApiResult<Response> {
    let tenant = hierarchy::create_tenant(&st, &ctx, &body).await?;
    Ok((StatusCode::CREATED, Json(tenant)).into_response())
}

pub async fn orgs_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<CreateOrgReq>,
) -> ApiResult<Response> {
    let org = hierarchy::create_org(&st, &ctx, tenant_id, &body).await?;
    Ok((StatusCode::CREATED, Json(org)).into_response())
}

pub async fn sites_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tenant_id, org_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateSiteReq>,
) -> ApiResult<Response> {
    let site = hierarchy::create_site(&st, &ctx, tenant_id, org_id, &body).await?;
    Ok((StatusCode::CREATED, Json(site)).into_response())
}

pub async fn groups_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tenant_id, site_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateGroupReq>,
) -> ApiResult<Response> {
    let group = hierarchy::create_group(&st, &ctx, tenant_id, site_id, &body).await?;
    Ok((StatusCode::CREATED, Json(group)).into_response())
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

pub async fn devices_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<DeviceListParams>,
) -> ApiResult<Json<ListResponse<Device>>> {
    Ok(Json(devices::list_devices(&st, &ctx, tid, &params).await?))
}

pub async fn devices_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateDeviceReq>,
) -> ApiResult<Response> {
    let device = devices::create_device(&st, &ctx, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(device)).into_response())
}

pub async fn devices_batch_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<BatchCreateDevicesReq>,
) -> ApiResult<Response> {
    let result = devices::batch_create_devices(&st, &ctx, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(result)).into_response())
}

pub async fn device_enrollment_batch_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, batch_id)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<DeviceEnrollmentBatchResponse>> {
    Ok(Json(
        devices::get_device_enrollment_batch(&st, &ctx, tid, batch_id).await?,
    ))
}

pub async fn devices_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Device>> {
    Ok(Json(devices::get_device(&st, &ctx, tid, did).await?))
}

pub async fn devices_patch(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateDeviceReq>,
) -> ApiResult<Json<Device>> {
    Ok(Json(devices::update_device(&st, &ctx, tid, did, &body).await?))
}

pub async fn devices_delete(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    devices::delete_device(&st, &ctx, tid, did).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn device_registration_code_rotate(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Device>> {
    Ok(Json(devices::rotate_registration_code(&st, &ctx, tid, did).await?))
}

pub async fn device_enrollment_token_issue(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<IssueDeviceEnrollmentTokenReq>,
) -> ApiResult<Response> {
    let token = devices::issue_device_enrollment_token(&st, &ctx, tid, did, &body).await?;
    Ok((StatusCode::CREATED, Json(token)).into_response())
}

pub async fn device_claim_with_enrollment_token(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<ClaimDeviceEnrollmentReq>,
) -> ApiResult<Response> {
    let device = devices::claim_device_with_enrollment_token(&st, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(device)).into_response())
}

// ---------------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------------

pub async fn policies_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<PolicyListParams>,
) -> ApiResult<Json<ListResponse<Policy>>> {
    Ok(Json(policies::list_policies(&st, &ctx, tid, &params).await?))
}

pub async fn policies_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreatePolicyReq>,
) -> ApiResult<Response> {
    let policy = policies::create_policy(&st, &ctx, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(policy)).into_response())
}

pub async fn policies_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Policy>> {
    Ok(Json(policies::get_policy(&st, &ctx, tid, pid).await?))
}

pub async fn policies_patch(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePolicyReq>,
) -> ApiResult<Json<Policy>> {
    Ok(Json(policies::update_policy(&st, &ctx, tid, pid, &body).await?))
}

pub async fn policies_delete(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    policies::delete_policy(&st, &ctx, tid, pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn policy_publish(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<PublishPolicyReq>,
) -> ApiResult<Response> {
    let revision = policies::publish_policy(&st, &ctx, tid, pid, body).await?;
    Ok((StatusCode::CREATED, Json(revision)).into_response())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub async fn commands_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    Ok(Json(commands::list_commands(&st, &ctx, tid, &params).await?))
}

pub async fn commands_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateCommandReq>,
) -> ApiResult<Response> {
    let command = commands::create_command(&st, &ctx, tid, &body).await?;
    Ok((StatusCode::ACCEPTED, Json(command)).into_response())
}

pub async fn commands_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Command>> {
    Ok(Json(commands::get_command(&st, &ctx, tid, cid).await?))
}

// ---------------------------------------------------------------------------
// Device Shadow
// ---------------------------------------------------------------------------

pub async fn shadow_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(shadow::get_shadow(&st, &ctx, tid, did).await?))
}

pub async fn shadow_update_desired(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowDesiredReq>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(
        shadow::update_shadow_desired(&st, &ctx, tid, did, &body).await?,
    ))
}

pub async fn shadow_update_reported(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowReportedReq>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(
        shadow::update_shadow_reported(&st, &ctx, tid, did, &body).await?,
    ))
}

// ---------------------------------------------------------------------------
// Device Actions (remote control convenience endpoint)
// ---------------------------------------------------------------------------

pub async fn device_action(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<DeviceActionReq>,
) -> ApiResult<Response> {
    let command = commands::create_device_action_command(&st, &ctx, tid, did, &body).await?;
    Ok((StatusCode::ACCEPTED, Json(command)).into_response())
}

// ---------------------------------------------------------------------------
// Device commands history (scoped to a single device)
// ---------------------------------------------------------------------------

pub async fn device_commands_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    Ok(Json(
        commands::list_device_commands(&st, &ctx, tid, did, &params).await?,
    ))
}

// ---------------------------------------------------------------------------
// Command result (get)
// ---------------------------------------------------------------------------

pub async fn command_result_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<dmsx_core::CommandResult>> {
    Ok(Json(commands::get_command_result(&st, &ctx, tid, cid).await?))
}

// ---------------------------------------------------------------------------
// Command lifecycle: update status + submit result
// ---------------------------------------------------------------------------

pub async fn command_status_update(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCommandStatusReq>,
) -> ApiResult<Json<Command>> {
    Ok(Json(
        commands::update_command_status(&st, &ctx, tid, cid, &body).await?,
    ))
}

pub async fn command_result_submit(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubmitCommandResultReq>,
) -> ApiResult<Response> {
    let result = commands::submit_command_result(&st, &ctx, tid, cid, &body).await?;
    Ok((StatusCode::CREATED, Json(result)).into_response())
}

pub async fn command_evidence_upload_token_issue(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<IssueEvidenceUploadTokenReq>,
) -> ApiResult<Response> {
    let token = commands::issue_evidence_upload_token(&st, &ctx, tid, cid, &body).await?;
    Ok((StatusCode::CREATED, Json(token)).into_response())
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

pub async fn artifacts_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<ArtifactListParams>,
) -> ApiResult<Json<ListResponse<Artifact>>> {
    Ok(Json(artifacts::list_artifacts(&st, &ctx, tid, &params).await?))
}

pub async fn artifacts_create(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateArtifactReq>,
) -> ApiResult<Response> {
    let artifact = artifacts::create_artifact(&st, &ctx, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(artifact)).into_response())
}

// ---------------------------------------------------------------------------
// Compliance
// ---------------------------------------------------------------------------

pub async fn compliance_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<FindingListParams>,
) -> ApiResult<Json<ListResponse<ComplianceFinding>>> {
    Ok(Json(compliance::list_findings(&st, &ctx, tid, &params).await?))
}

// ---------------------------------------------------------------------------
// Admin / Observability / Config
// ---------------------------------------------------------------------------

pub async fn audit_logs_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Query(params): Query<AuditLogListParams>,
) -> ApiResult<Json<ListResponse<crate::dto::AuditLog>>> {
    Ok(Json(audit::list_audit_logs(&st, &ctx, tid, &params).await?))
}

pub async fn system_settings_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(key): Path<String>,
) -> ApiResult<Json<SystemSetting>> {
    let setting = system_settings::get_global_setting(&st, &ctx, &key)
        .await?
        .ok_or_else(|| DmsxError::NotFound(format!("system setting '{key}'")))?;
    Ok(Json(setting))
}

pub async fn system_settings_put(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(key): Path<String>,
    Json(body): Json<SystemSettingUpsertReq>,
) -> ApiResult<Json<SystemSetting>> {
    Ok(Json(
        system_settings::upsert_global_setting(&st, &ctx, &key, body).await?,
    ))
}

pub async fn rbac_roles_list(
    Extension(_ctx): Extension<AuthContext>,
) -> ApiResult<Json<Vec<RbacRole>>> {
    Ok(Json(vec![
        RbacRole {
            name: "PlatformAdmin".to_string(),
            scope: "platform".to_string(),
            description: "平台级完全管理权限，可读写所有平台与租户资源。".to_string(),
            platform_read: true,
            platform_write: true,
            tenant_read: true,
            tenant_write: true,
        },
        RbacRole {
            name: "PlatformViewer".to_string(),
            scope: "platform".to_string(),
            description: "平台级只读权限，可查看平台配置、租户目录、全局审计与平台健康。".to_string(),
            platform_read: true,
            platform_write: false,
            tenant_read: false,
            tenant_write: false,
        },
        RbacRole {
            name: "TenantAdmin".to_string(),
            scope: "tenant".to_string(),
            description: "租户级完全管理权限，可读写当前租户下的大多数资源。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
        },
        RbacRole {
            name: "SiteAdmin".to_string(),
            scope: "tenant".to_string(),
            description: "租户级运维管理角色，可管理设备、命令、影子与远程桌面；策略、制品、AI 仅只读。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
        },
        RbacRole {
            name: "Operator".to_string(),
            scope: "tenant".to_string(),
            description: "租户级操作员角色，可执行设备与命令相关操作；策略、制品、AI、合规、统计仅只读。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: true,
        },
        RbacRole {
            name: "Auditor".to_string(),
            scope: "tenant".to_string(),
            description: "租户级审计角色，仅允许只读访问，不允许远程桌面与 AI。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: false,
        },
        RbacRole {
            name: "ReadOnly".to_string(),
            scope: "tenant".to_string(),
            description: "租户级只读角色，可查看常规租户资源，不允许写操作、远程桌面与 AI。".to_string(),
            platform_read: false,
            platform_write: false,
            tenant_read: true,
            tenant_write: false,
        },
    ]))
}

pub async fn platform_tenants_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Query(params): Query<PlatformTenantListParams>,
) -> ApiResult<Json<ListResponse<PlatformTenantSummary>>> {
    Ok(Json(platform::list_tenants_paginated(&st, &ctx, &params).await?))
}

pub async fn platform_audit_logs_list(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Query(params): Query<AuditLogListParams>,
) -> ApiResult<Json<ListResponse<crate::dto::AuditLog>>> {
    Ok(Json(platform::list_platform_audit_logs(&st, &ctx, &params).await?))
}

pub async fn platform_health_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<PlatformHealth>> {
    Ok(Json(platform::platform_health(&st, &ctx).await?))
}

pub async fn platform_quotas_get(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
) -> ApiResult<Json<ListResponse<PlatformQuota>>> {
    Ok(Json(platform::platform_quotas(&st, &ctx).await?))
}

pub async fn policy_editor_create_and_publish(
    State(st): State<AppState>,
    Extension(ctx): Extension<AuthContext>,
    Path(tid): Path<Uuid>,
    Json(body): Json<PolicyEditorPublishReq>,
) -> ApiResult<Response> {
    body.validate()?;

    let PolicyEditorPublishReq {
        name,
        description,
        scope_kind,
        scope_expr,
    } = body;

    let create_req = CreatePolicyReq {
        name,
        description,
        scope_kind,
    };

    let policy = policies::create_policy(&st, &ctx, tid, &create_req).await?;

    // Persist the full `scope_expr` into the revision `spec` so downstream evaluation
    // can be enabled later without requiring additional schema changes.
    let spec = json!({
        "scope_kind": policy.scope_kind,
        "scope_expr": scope_expr,
    });

    let revision = policies::publish_policy(
        &st,
        &ctx,
        tid,
        policy.id.0,
        PublishPolicyReq { spec },
    )
    .await?;

    Ok((StatusCode::CREATED, Json(revision)).into_response())
}

// ---------------------------------------------------------------------------
// AI handlers (delegate to dmsx-ai crate)
// ---------------------------------------------------------------------------

pub async fn ai_anomaly_detect(
    Extension(_ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<dmsx_ai::AnomalyDetectionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use dmsx_ai::{anomaly::RuleBasedAnomalyDetector, engine::AiEngine};
    let reports = RuleBasedAnomalyDetector
        .detect_anomalies(&body)
        .await
        .map_err(|e| DmsxError::Internal(e.to_string()))?;
    Ok(Json(
        json!({ "tenant_id": tenant_id, "anomalies": reports }),
    ))
}

pub async fn ai_recommend_policies(
    Extension(_ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<dmsx_ai::PolicyRecommendationRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use dmsx_ai::{anomaly::RuleBasedAnomalyDetector, engine::AiEngine};
    let recs = RuleBasedAnomalyDetector
        .recommend_policies(&body)
        .await
        .map_err(|e| DmsxError::Internal(e.to_string()))?;
    Ok(Json(
        json!({ "tenant_id": tenant_id, "recommendations": recs }),
    ))
}

pub async fn ai_chat(
    Extension(_ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<dmsx_ai::AssistantChatRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use dmsx_ai::{anomaly::RuleBasedAnomalyDetector, engine::AiEngine};
    match RuleBasedAnomalyDetector.chat(&body).await {
        Ok(resp) => Ok(Json(json!({ "tenant_id": tenant_id, "response": resp }))),
        Err(dmsx_ai::engine::AiError::ModelUnavailable(msg)) => Ok(Json(json!({
            "tenant_id": tenant_id,
            "response": { "reply": msg, "actions": [], "references": [] }
        }))),
        Err(e) => Err(DmsxError::Internal(e.to_string())),
    }
}

pub async fn ai_predict_maintenance(
    Extension(_ctx): Extension<AuthContext>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<dmsx_ai::PredictionRequest>,
) -> ApiResult<Json<serde_json::Value>> {
    use dmsx_ai::{anomaly::RuleBasedAnomalyDetector, engine::AiEngine};
    let preds = RuleBasedAnomalyDetector
        .predict_maintenance(&body)
        .await
        .map_err(|e| DmsxError::Internal(e.to_string()))?;
    Ok(Json(
        json!({ "tenant_id": tenant_id, "predictions": preds }),
    ))
}
