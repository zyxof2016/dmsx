use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{DateTime, Duration, Utc};
use dmsx_core::{Command, CommandResult, CommandStatus, DmsxError};
use hmac::{Hmac, Mac};
use serde_json::{json, Value};
use sha2::Sha256;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{
    CommandListParams, CreateCommandReq, DeviceActionReq, EvidenceUploadToken,
    IssueEvidenceUploadTokenReq, ListResponse, SubmitCommandResultReq, UpdateCommandStatusReq,
};
use crate::error::map_db_error;
use crate::helpers::command_status_from_exit_code;
use crate::repo::{audit, commands as command_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

fn upload_token_secret(st: &AppState) -> ServiceResult<&str> {
    st.upload_token_hmac_secret.as_deref().ok_or_else(|| {
        DmsxError::Internal(
            "evidence upload token signing is not configured (missing DMSX_API_UPLOAD_TOKEN_HMAC_SECRET)"
                .into(),
        )
    })
}

fn sign_upload_token(payload: &Value, secret: &str) -> ServiceResult<String> {
    let payload_raw = serde_json::to_vec(payload)
        .map_err(|e| DmsxError::Internal(format!("serialize upload token payload: {e}")))?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_raw);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| DmsxError::Internal("upload token hmac init".into()))?;
    mac.update(payload_b64.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("v1.{payload_b64}.{sig_b64}"))
}

pub async fn list_commands(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &CommandListParams,
) -> ServiceResult<ListResponse<Command>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = command_repo::list_commands(&mut *tx, tid, params)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

pub async fn create_command(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &CreateCommandReq,
) -> ServiceResult<Command> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::create_command(&mut *tx, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "command",
        &command.id.0.to_string(),
        json!({"target_device_id": body.target_device_id}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    if let Some(js) = &st.command_jetstream {
        js.publish_command_created(&command);
    }
    Ok(command)
}

pub async fn get_command(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
) -> ServiceResult<Command> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::get_command(&mut *tx, tid, cid)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("command {cid}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(command)
}

pub async fn issue_evidence_upload_token(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
    body: &IssueEvidenceUploadTokenReq,
) -> ServiceResult<EvidenceUploadToken> {
    body.validate()?;
    let secret = upload_token_secret(st)?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::get_command(&mut *tx, tid, cid)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("command {cid}")))?;

    let expires_at: DateTime<Utc> = Utc::now() + Duration::seconds(body.expires_in_seconds());
    let payload = json!({
        "tenant_id": tid,
        "device_id": command.target_device_id.0,
        "command_id": cid,
        "exp": expires_at.timestamp(),
        "content_type": body.content_type,
    });
    let upload_token = sign_upload_token(&payload, secret)?;

    audit::write_audit(
        &mut *tx,
        tid,
        "issue_evidence_upload_token",
        "command",
        &cid.to_string(),
        json!({
            "device_id": command.target_device_id.0,
            "expires_at": expires_at,
            "content_type": body.content_type
        }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;

    Ok(EvidenceUploadToken {
        upload_token,
        tenant_id: tid,
        device_id: command.target_device_id.0,
        command_id: cid,
        content_type: body.content_type.clone(),
        expires_at,
    })
}

pub async fn create_device_action_command(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
    body: &DeviceActionReq,
) -> ServiceResult<Command> {
    body.validate()?;
    let payload = json!({
        "action": body.action.clone(),
        "params": body.params.clone()
    });
    let command_req = CreateCommandReq {
        target_device_id: did,
        payload,
        priority: body.priority,
        ttl_seconds: body.ttl_seconds,
        idempotency_key: None,
    };
    command_req.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::create_command(&mut *tx, tid, &command_req)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "device_action",
        "command",
        &command.id.0.to_string(),
        json!({"device_id": did, "action": &body.action}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    if let Some(js) = &st.command_jetstream {
        js.publish_command_created(&command);
    }
    Ok(command)
}

pub async fn list_device_commands(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
    params: &CommandListParams,
) -> ServiceResult<ListResponse<Command>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = command_repo::list_device_commands(&mut *tx, tid, did, lim, off)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

pub async fn get_command_result(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
) -> ServiceResult<CommandResult> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let result = command_repo::get_command_result(&mut *tx, tid, cid)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("result for command {cid}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(result)
}

pub async fn update_command_status(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
    body: &UpdateCommandStatusReq,
) -> ServiceResult<Command> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let command = command_repo::update_command_status(&mut *tx, tid, cid, body.status)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("command {cid}")))?;
    audit::write_audit(
        &mut *tx,
        tid,
        "update_status",
        "command",
        &cid.to_string(),
        json!({"new_status": format!("{:?}", body.status)}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(command)
}

pub async fn submit_command_result(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
    body: &SubmitCommandResultReq,
) -> ServiceResult<CommandResult> {
    submit_command_result_with_status(st, ctx, tid, cid, body, None).await
}

pub async fn submit_command_result_with_status(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    cid: Uuid,
    body: &SubmitCommandResultReq,
    explicit_status: Option<CommandStatus>,
) -> ServiceResult<CommandResult> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let result = command_repo::upsert_command_result(
        &mut *tx,
        tid,
        cid,
        body.exit_code,
        &body.stdout,
        &body.stderr,
        body.evidence_key.as_deref(),
    )
    .await
    .map_err(map_db_error)?;

    let new_status = explicit_status.unwrap_or_else(|| command_status_from_exit_code(body.exit_code));
    let _ = command_repo::update_command_status(&mut *tx, tid, cid, new_status)
        .await
        .map_err(map_db_error);
    audit::write_audit(
        &mut *tx,
        tid,
        "submit_result",
        "command",
        &cid.to_string(),
        json!({"exit_code": body.exit_code, "status": format!("{:?}", new_status)}),
    )
    .await
    .ok();

    tx.commit().await.map_err(map_db_error)?;
    Ok(result)
}
