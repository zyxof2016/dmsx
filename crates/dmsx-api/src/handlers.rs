use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use dmsx_core::*;
use serde_json::json;
use uuid::Uuid;

use crate::dto::*;
use crate::services::{
    artifacts, commands, compliance, devices, hierarchy, policies, shadow, stats,
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
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<DashboardStats>> {
    Ok(Json(stats::get_stats(&st, tenant_id).await?))
}

// ---------------------------------------------------------------------------
// Tenant hierarchy
// ---------------------------------------------------------------------------

pub async fn tenants_create(
    State(st): State<AppState>,
    Json(body): Json<CreateTenantReq>,
) -> ApiResult<Response> {
    let tenant = hierarchy::create_tenant(&st, &body).await?;
    Ok((StatusCode::CREATED, Json(tenant)).into_response())
}

pub async fn orgs_create(
    State(st): State<AppState>,
    Path(tenant_id): Path<Uuid>,
    Json(body): Json<CreateOrgReq>,
) -> ApiResult<Response> {
    let org = hierarchy::create_org(&st, tenant_id, &body).await?;
    Ok((StatusCode::CREATED, Json(org)).into_response())
}

pub async fn sites_create(
    State(st): State<AppState>,
    Path((tenant_id, org_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateSiteReq>,
) -> ApiResult<Response> {
    let site = hierarchy::create_site(&st, tenant_id, org_id, &body).await?;
    Ok((StatusCode::CREATED, Json(site)).into_response())
}

pub async fn groups_create(
    State(st): State<AppState>,
    Path((tenant_id, site_id)): Path<(Uuid, Uuid)>,
    Json(body): Json<CreateGroupReq>,
) -> ApiResult<Response> {
    let group = hierarchy::create_group(&st, tenant_id, site_id, &body).await?;
    Ok((StatusCode::CREATED, Json(group)).into_response())
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

pub async fn devices_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<DeviceListParams>,
) -> ApiResult<Json<ListResponse<Device>>> {
    Ok(Json(devices::list_devices(&st, tid, &params).await?))
}

pub async fn devices_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateDeviceReq>,
) -> ApiResult<Response> {
    let device = devices::create_device(&st, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(device)).into_response())
}

pub async fn devices_get(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Device>> {
    Ok(Json(devices::get_device(&st, tid, did).await?))
}

pub async fn devices_patch(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateDeviceReq>,
) -> ApiResult<Json<Device>> {
    Ok(Json(devices::update_device(&st, tid, did, &body).await?))
}

pub async fn devices_delete(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    devices::delete_device(&st, tid, did).await?;
    Ok(StatusCode::NO_CONTENT)
}

// ---------------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------------

pub async fn policies_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<PolicyListParams>,
) -> ApiResult<Json<ListResponse<Policy>>> {
    Ok(Json(policies::list_policies(&st, tid, &params).await?))
}

pub async fn policies_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreatePolicyReq>,
) -> ApiResult<Response> {
    let policy = policies::create_policy(&st, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(policy)).into_response())
}

pub async fn policies_get(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Policy>> {
    Ok(Json(policies::get_policy(&st, tid, pid).await?))
}

pub async fn policies_patch(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePolicyReq>,
) -> ApiResult<Json<Policy>> {
    Ok(Json(policies::update_policy(&st, tid, pid, &body).await?))
}

pub async fn policies_delete(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    policies::delete_policy(&st, tid, pid).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub async fn policy_publish(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<PublishPolicyReq>,
) -> ApiResult<Response> {
    let revision = policies::publish_policy(&st, tid, pid, body).await?;
    Ok((StatusCode::CREATED, Json(revision)).into_response())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub async fn commands_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    Ok(Json(commands::list_commands(&st, tid, &params).await?))
}

pub async fn commands_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateCommandReq>,
) -> ApiResult<Response> {
    let command = commands::create_command(&st, tid, &body).await?;
    Ok((StatusCode::ACCEPTED, Json(command)).into_response())
}

pub async fn commands_get(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Command>> {
    Ok(Json(commands::get_command(&st, tid, cid).await?))
}

// ---------------------------------------------------------------------------
// Device Shadow
// ---------------------------------------------------------------------------

pub async fn shadow_get(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(shadow::get_shadow(&st, tid, did).await?))
}

pub async fn shadow_update_desired(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowDesiredReq>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(
        shadow::update_shadow_desired(&st, tid, did, &body).await?,
    ))
}

pub async fn shadow_update_reported(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowReportedReq>,
) -> ApiResult<Json<ShadowResponse>> {
    Ok(Json(
        shadow::update_shadow_reported(&st, tid, did, &body).await?,
    ))
}

// ---------------------------------------------------------------------------
// Device Actions (remote control convenience endpoint)
// ---------------------------------------------------------------------------

pub async fn device_action(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<DeviceActionReq>,
) -> ApiResult<Response> {
    let command = commands::create_device_action_command(&st, tid, did, &body).await?;
    Ok((StatusCode::ACCEPTED, Json(command)).into_response())
}

// ---------------------------------------------------------------------------
// Device commands history (scoped to a single device)
// ---------------------------------------------------------------------------

pub async fn device_commands_list(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    Ok(Json(commands::list_device_commands(&st, tid, did, &params).await?))
}

// ---------------------------------------------------------------------------
// Command result (get)
// ---------------------------------------------------------------------------

pub async fn command_result_get(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<dmsx_core::CommandResult>> {
    Ok(Json(commands::get_command_result(&st, tid, cid).await?))
}

// ---------------------------------------------------------------------------
// Command lifecycle: update status + submit result
// ---------------------------------------------------------------------------

pub async fn command_status_update(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCommandStatusReq>,
) -> ApiResult<Json<Command>> {
    Ok(Json(
        commands::update_command_status(&st, tid, cid, &body).await?,
    ))
}

pub async fn command_result_submit(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubmitCommandResultReq>,
) -> ApiResult<Response> {
    let result = commands::submit_command_result(&st, tid, cid, &body).await?;
    Ok((StatusCode::CREATED, Json(result)).into_response())
}

// ---------------------------------------------------------------------------
// Artifacts
// ---------------------------------------------------------------------------

pub async fn artifacts_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<ArtifactListParams>,
) -> ApiResult<Json<ListResponse<Artifact>>> {
    Ok(Json(artifacts::list_artifacts(&st, tid, &params).await?))
}

pub async fn artifacts_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateArtifactReq>,
) -> ApiResult<Response> {
    let artifact = artifacts::create_artifact(&st, tid, &body).await?;
    Ok((StatusCode::CREATED, Json(artifact)).into_response())
}

// ---------------------------------------------------------------------------
// Compliance
// ---------------------------------------------------------------------------

pub async fn compliance_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<FindingListParams>,
) -> ApiResult<Json<ListResponse<ComplianceFinding>>> {
    Ok(Json(compliance::list_findings(&st, tid, &params).await?))
}

// ---------------------------------------------------------------------------
// AI handlers (delegate to dmsx-ai crate)
// ---------------------------------------------------------------------------

pub async fn ai_anomaly_detect(
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
