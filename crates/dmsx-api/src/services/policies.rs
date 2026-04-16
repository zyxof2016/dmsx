use dmsx_core::{DmsxError, Policy, PolicyRevision};
use serde_json::json;
use uuid::Uuid;

use crate::dto::{CreatePolicyReq, ListResponse, PolicyListParams, PublishPolicyReq, UpdatePolicyReq};
use crate::error::map_db_error;
use crate::repo::{audit, policies as policy_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_policies(
    st: &AppState,
    tid: Uuid,
    params: &PolicyListParams,
) -> ServiceResult<ListResponse<Policy>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = policy_repo::list_policies(&st.db, tid, params)
        .await
        .map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

pub async fn create_policy(
    st: &AppState,
    tid: Uuid,
    body: &CreatePolicyReq,
) -> ServiceResult<Policy> {
    body.validate()?;
    let policy = policy_repo::create_policy(&st.db, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "policy",
        &policy.id.0.to_string(),
        json!({"name": &body.name}),
    )
    .await
    .ok();
    Ok(policy)
}

pub async fn get_policy(st: &AppState, tid: Uuid, pid: Uuid) -> ServiceResult<Policy> {
    policy_repo::get_policy(&st.db, tid, pid)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))
}

pub async fn update_policy(
    st: &AppState,
    tid: Uuid,
    pid: Uuid,
    body: &UpdatePolicyReq,
) -> ServiceResult<Policy> {
    body.validate()?;
    let policy = policy_repo::update_policy(&st.db, tid, pid, body)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("policy {pid}")))?;
    audit::write_audit(&st.db, tid, "update", "policy", &pid.to_string(), json!({}))
        .await
        .ok();
    Ok(policy)
}

pub async fn delete_policy(st: &AppState, tid: Uuid, pid: Uuid) -> ServiceResult<()> {
    if policy_repo::delete_policy(&st.db, tid, pid)
        .await
        .map_err(map_db_error)?
    {
        audit::write_audit(&st.db, tid, "delete", "policy", &pid.to_string(), json!({}))
            .await
            .ok();
        Ok(())
    } else {
        Err(DmsxError::NotFound(format!("policy {pid}")))
    }
}

pub async fn publish_policy(
    st: &AppState,
    tid: Uuid,
    pid: Uuid,
    body: PublishPolicyReq,
) -> ServiceResult<PolicyRevision> {
    let revision = policy_repo::publish_policy(&st.db, tid, pid, body.spec)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "publish",
        "policy_revision",
        &pid.to_string(),
        json!({"version": revision.version}),
    )
    .await
    .ok();
    Ok(revision)
}
