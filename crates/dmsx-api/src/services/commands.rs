use dmsx_core::{Command, CommandResult, DmsxError};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{
    CommandListParams, CreateCommandReq, DeviceActionReq, ListResponse, SubmitCommandResultReq,
    UpdateCommandStatusReq,
};
use crate::error::map_db_error;
use crate::helpers::command_status_from_exit_code;
use crate::repo::{audit, commands as command_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

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

    let new_status = command_status_from_exit_code(body.exit_code);
    let _ = command_repo::update_command_status(&mut *tx, tid, cid, new_status)
        .await
        .map_err(map_db_error);
    audit::write_audit(
        &mut *tx,
        tid,
        "submit_result",
        "command",
        &cid.to_string(),
        json!({"exit_code": body.exit_code}),
    )
    .await
    .ok();

    tx.commit().await.map_err(map_db_error)?;
    Ok(result)
}
