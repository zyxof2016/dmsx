use dmsx_core::{DmsxError, Policy, PolicyRevision};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{CreatePolicyReq, ListResponse, PolicyListParams, PublishPolicyReq, UpdatePolicyReq};
use crate::error::map_db_error;
use crate::repo::{audit, policies as policy_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_policies(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &PolicyListParams,
) -> ServiceResult<ListResponse<Policy>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = policy_repo::list_policies(&mut *tx, tid, params)
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

pub async fn create_policy(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &CreatePolicyReq,
) -> ServiceResult<Policy> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let policy = policy_repo::create_policy(&mut *tx, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "policy",
        &policy.id.0.to_string(),
        json!({"name": &body.name}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(policy)
}

pub async fn get_policy(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    pid: Uuid,
) -> ServiceResult<Policy> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let policy = policy_repo::get_policy(&mut *tx, tid, pid)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(policy)
}

pub async fn update_policy(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    pid: Uuid,
    body: &UpdatePolicyReq,
) -> ServiceResult<Policy> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let policy = policy_repo::update_policy(&mut *tx, tid, pid, body)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))?;
    audit::write_audit(&mut *tx, tid, "update", "policy", &pid.to_string(), json!({}))
        .await
        .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(policy)
}

pub async fn delete_policy(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    pid: Uuid,
) -> ServiceResult<()> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    if policy_repo::delete_policy(&mut *tx, tid, pid)
        .await
        .map_err(map_db_error)?
    {
        audit::write_audit(&mut *tx, tid, "delete", "policy", &pid.to_string(), json!({}))
            .await
            .ok();
        tx.commit().await.map_err(map_db_error)?;
        Ok(())
    } else {
        tx.commit().await.map_err(map_db_error)?;
        Err(DmsxError::NotFound(format!("policy {pid}")))
    }
}

pub async fn publish_policy(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    pid: Uuid,
    body: PublishPolicyReq,
) -> ServiceResult<PolicyRevision> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let revision = policy_repo::publish_policy(&mut *tx, tid, pid, body.spec)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "publish",
        "policy_revision",
        &pid.to_string(),
        json!({"version": revision.version}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(revision)
}
