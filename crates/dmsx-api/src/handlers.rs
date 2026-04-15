use axum::{
    extract::{Path, Query, Request, State},
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use dmsx_core::*;
use serde_json::json;
use uuid::Uuid;

use crate::db;
use crate::dto::*;
use crate::state::AppState;

pub type ApiResult<T> = Result<T, DmsxError>;

// ---------------------------------------------------------------------------
// Error mapping (refined: no internal detail leaks)
// ---------------------------------------------------------------------------

fn db_err(e: sqlx::Error) -> DmsxError {
    match &e {
        sqlx::Error::Database(dbe) => {
            let code = dbe.code().unwrap_or_default();
            match code.as_ref() {
                "23505" => DmsxError::Conflict("resource already exists".into()),
                "23503" => DmsxError::Validation("referenced resource does not exist".into()),
                "23514" => DmsxError::Validation("check constraint violated".into()),
                _ => {
                    tracing::error!(pg_code = %code, "unhandled database error: {e}");
                    DmsxError::Internal("database error".into())
                }
            }
        }
        _ => {
            tracing::error!("database error: {e}");
            DmsxError::Internal("database error".into())
        }
    }
}

// ---------------------------------------------------------------------------
// Auth middleware (stub)
// ---------------------------------------------------------------------------

pub async fn auth_middleware(request: Request, next: Next) -> Response {
    next.run(request).await
}

// ---------------------------------------------------------------------------
// Health
// ---------------------------------------------------------------------------

pub async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok", "service": "dmsx-api" }))
}

// ---------------------------------------------------------------------------
// Dashboard Stats
// ---------------------------------------------------------------------------

pub async fn stats(
    State(st): State<AppState>,
    Path(tenant_id): Path<Uuid>,
) -> ApiResult<Json<DashboardStats>> {
    let s = db::get_stats(&st.db, tenant_id).await.map_err(db_err)?;
    Ok(Json(s))
}

// ---------------------------------------------------------------------------
// Devices
// ---------------------------------------------------------------------------

pub async fn devices_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<DeviceListParams>,
) -> ApiResult<Json<ListResponse<Device>>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_devices(&st.db, tid, &params).await.map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
}

pub async fn devices_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateDeviceReq>,
) -> ApiResult<Response> {
    body.validate()?;
    let d = db::create_device(&st.db, tid, &body).await.map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "create",
        "device",
        &d.id.0.to_string(),
        json!({"platform": format!("{:?}", body.platform), "hostname": body.hostname}),
    )
    .await
    .ok();
    Ok((StatusCode::CREATED, Json(d)).into_response())
}

pub async fn devices_get(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Device>> {
    db::get_device(&st.db, tid, did)
        .await
        .map_err(db_err)?
        .map(Json)
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))
}

pub async fn devices_patch(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateDeviceReq>,
) -> ApiResult<Json<Device>> {
    body.validate()?;
    let d = db::update_device(&st.db, tid, did, &body)
        .await
        .map_err(db_err)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    db::write_audit(&st.db, tid, "update", "device", &did.to_string(), json!({}))
        .await
        .ok();
    Ok(Json(d))
}

pub async fn devices_delete(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    if db::delete_device(&st.db, tid, did).await.map_err(db_err)? {
        db::write_audit(&st.db, tid, "delete", "device", &did.to_string(), json!({}))
            .await
            .ok();
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(DmsxError::NotFound(format!("device {did}")))
    }
}

// ---------------------------------------------------------------------------
// Policies
// ---------------------------------------------------------------------------

pub async fn policies_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<PolicyListParams>,
) -> ApiResult<Json<ListResponse<Policy>>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_policies(&st.db, tid, &params).await.map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
}

pub async fn policies_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreatePolicyReq>,
) -> ApiResult<Response> {
    body.validate()?;
    let p = db::create_policy(&st.db, tid, &body).await.map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "create",
        "policy",
        &p.id.0.to_string(),
        json!({"name": body.name}),
    )
    .await
    .ok();
    Ok((StatusCode::CREATED, Json(p)).into_response())
}

pub async fn policies_get(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Policy>> {
    db::get_policy(&st.db, tid, pid)
        .await
        .map_err(db_err)?
        .map(Json)
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))
}

pub async fn policies_patch(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdatePolicyReq>,
) -> ApiResult<Json<Policy>> {
    body.validate()?;
    let p = db::update_policy(&st.db, tid, pid, &body)
        .await
        .map_err(db_err)?
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))?;
    db::write_audit(&st.db, tid, "update", "policy", &pid.to_string(), json!({}))
        .await
        .ok();
    Ok(Json(p))
}

pub async fn policies_delete(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
) -> ApiResult<StatusCode> {
    if db::delete_policy(&st.db, tid, pid).await.map_err(db_err)? {
        db::write_audit(&st.db, tid, "delete", "policy", &pid.to_string(), json!({}))
            .await
            .ok();
        Ok(StatusCode::NO_CONTENT)
    } else {
        Err(DmsxError::NotFound(format!("policy {pid}")))
    }
}

pub async fn policy_publish(
    State(st): State<AppState>,
    Path((tid, pid)): Path<(Uuid, Uuid)>,
    Json(body): Json<PublishPolicyReq>,
) -> ApiResult<Response> {
    let rev = db::publish_policy(&st.db, tid, pid, body.spec)
        .await
        .map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "publish",
        "policy_revision",
        &pid.to_string(),
        json!({"version": rev.version}),
    )
    .await
    .ok();
    Ok((StatusCode::CREATED, Json(rev)).into_response())
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

pub async fn commands_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_commands(&st.db, tid, &params).await.map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
}

pub async fn commands_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateCommandReq>,
) -> ApiResult<Response> {
    body.validate()?;
    let c = db::create_command(&st.db, tid, &body)
        .await
        .map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "create",
        "command",
        &c.id.0.to_string(),
        json!({"target_device_id": body.target_device_id}),
    )
    .await
    .ok();
    Ok((StatusCode::ACCEPTED, Json(c)).into_response())
}

pub async fn commands_get(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<Command>> {
    db::get_command(&st.db, tid, cid)
        .await
        .map_err(db_err)?
        .map(Json)
        .ok_or_else(|| DmsxError::NotFound(format!("command {cid}")))
}

// ---------------------------------------------------------------------------
// Device Shadow
// ---------------------------------------------------------------------------

fn compute_delta(
    desired: &serde_json::Value,
    reported: &serde_json::Value,
) -> serde_json::Value {
    let mut delta = serde_json::Map::new();
    if let Some(d_obj) = desired.as_object() {
        let r_obj = reported.as_object();
        for (k, v) in d_obj {
            let differs = r_obj.map_or(true, |r| r.get(k) != Some(v));
            if differs {
                delta.insert(k.clone(), v.clone());
            }
        }
    }
    serde_json::Value::Object(delta)
}

pub async fn shadow_get(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<ShadowResponse>> {
    let s = db::get_or_create_shadow(&st.db, tid, did)
        .await
        .map_err(db_err)?;
    let delta = compute_delta(&s.desired, &s.reported);
    Ok(Json(ShadowResponse {
        device_id: did,
        reported: s.reported,
        desired: s.desired,
        delta,
        reported_at: s.reported_at,
        desired_at: s.desired_at,
        version: s.version,
    }))
}

pub async fn shadow_update_desired(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowDesiredReq>,
) -> ApiResult<Json<ShadowResponse>> {
    body.validate()?;
    let s = db::update_shadow_desired(&st.db, tid, did, &body.desired)
        .await
        .map_err(db_err)?;
    db::write_audit(&st.db, tid, "update_desired", "device_shadow", &did.to_string(), json!({}))
        .await
        .ok();
    let delta = compute_delta(&s.desired, &s.reported);
    Ok(Json(ShadowResponse {
        device_id: did,
        reported: s.reported,
        desired: s.desired,
        delta,
        reported_at: s.reported_at,
        desired_at: s.desired_at,
        version: s.version,
    }))
}

pub async fn shadow_update_reported(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateShadowReportedReq>,
) -> ApiResult<Json<ShadowResponse>> {
    body.validate()?;
    let s = db::update_shadow_reported(&st.db, tid, did, &body.reported)
        .await
        .map_err(db_err)?;
    let delta = compute_delta(&s.desired, &s.reported);
    Ok(Json(ShadowResponse {
        device_id: did,
        reported: s.reported,
        desired: s.desired,
        delta,
        reported_at: s.reported_at,
        desired_at: s.desired_at,
        version: s.version,
    }))
}

// ---------------------------------------------------------------------------
// Device Actions (remote control convenience endpoint)
// ---------------------------------------------------------------------------

pub async fn device_action(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Json(body): Json<DeviceActionReq>,
) -> ApiResult<Response> {
    body.validate()?;
    let payload = json!({ "action": body.action, "params": body.params });
    let cmd_req = CreateCommandReq {
        target_device_id: did,
        payload,
        priority: body.priority,
        ttl_seconds: body.ttl_seconds,
        idempotency_key: None,
    };
    cmd_req.validate()?;
    let c = db::create_command(&st.db, tid, &cmd_req)
        .await
        .map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "device_action",
        "command",
        &c.id.0.to_string(),
        json!({"device_id": did, "action": body.action}),
    )
    .await
    .ok();
    Ok((StatusCode::ACCEPTED, Json(c)).into_response())
}

// ---------------------------------------------------------------------------
// Device commands history (scoped to a single device)
// ---------------------------------------------------------------------------

pub async fn device_commands_list(
    State(st): State<AppState>,
    Path((tid, did)): Path<(Uuid, Uuid)>,
    Query(params): Query<CommandListParams>,
) -> ApiResult<Json<ListResponse<Command>>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_device_commands(&st.db, tid, did, lim, off)
        .await
        .map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
}

// ---------------------------------------------------------------------------
// Command result (get)
// ---------------------------------------------------------------------------

pub async fn command_result_get(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
) -> ApiResult<Json<dmsx_core::CommandResult>> {
    db::get_command_result(&st.db, tid, cid)
        .await
        .map_err(db_err)?
        .map(Json)
        .ok_or_else(|| DmsxError::NotFound(format!("result for command {cid}")))
}

// ---------------------------------------------------------------------------
// Command lifecycle: update status + submit result
// ---------------------------------------------------------------------------

pub async fn command_status_update(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<UpdateCommandStatusReq>,
) -> ApiResult<Json<Command>> {
    let c = db::update_command_status(&st.db, tid, cid, body.status)
        .await
        .map_err(db_err)?
        .ok_or_else(|| DmsxError::NotFound(format!("command {cid}")))?;
    db::write_audit(
        &st.db,
        tid,
        "update_status",
        "command",
        &cid.to_string(),
        json!({"new_status": format!("{:?}", body.status)}),
    )
    .await
    .ok();
    Ok(Json(c))
}

pub async fn command_result_submit(
    State(st): State<AppState>,
    Path((tid, cid)): Path<(Uuid, Uuid)>,
    Json(body): Json<SubmitCommandResultReq>,
) -> ApiResult<Response> {
    let result = db::upsert_command_result(
        &st.db,
        tid,
        cid,
        body.exit_code,
        &body.stdout,
        &body.stderr,
        body.evidence_key.as_deref(),
    )
    .await
    .map_err(db_err)?;
    let new_status = if body.exit_code.unwrap_or(-1) == 0 {
        CommandStatus::Succeeded
    } else {
        CommandStatus::Failed
    };
    db::update_command_status(&st.db, tid, cid, new_status)
        .await
        .map_err(db_err)
        .ok();
    db::write_audit(
        &st.db,
        tid,
        "submit_result",
        "command",
        &cid.to_string(),
        json!({"exit_code": body.exit_code}),
    )
    .await
    .ok();
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
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_artifacts(&st.db, tid, &params).await.map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
}

pub async fn artifacts_create(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Json(body): Json<CreateArtifactReq>,
) -> ApiResult<Response> {
    body.validate()?;
    let a = db::create_artifact(&st.db, tid, &body)
        .await
        .map_err(db_err)?;
    db::write_audit(
        &st.db,
        tid,
        "create",
        "artifact",
        &a.id.0.to_string(),
        json!({"name": body.name, "version": body.version}),
    )
    .await
    .ok();
    Ok((StatusCode::CREATED, Json(a)).into_response())
}

// ---------------------------------------------------------------------------
// Compliance
// ---------------------------------------------------------------------------

pub async fn compliance_list(
    State(st): State<AppState>,
    Path(tid): Path<Uuid>,
    Query(params): Query<FindingListParams>,
) -> ApiResult<Json<ListResponse<ComplianceFinding>>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = db::list_findings(&st.db, tid, &params).await.map_err(db_err)?;
    Ok(Json(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    }))
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
