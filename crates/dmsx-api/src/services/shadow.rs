use dmsx_core::DeviceShadow;
use serde_json::json;
use uuid::Uuid;

use crate::dto::{ShadowResponse, UpdateShadowDesiredReq, UpdateShadowReportedReq};
use crate::error::map_db_error;
use crate::helpers::compute_shadow_delta;
use crate::repo::{audit, shadow as shadow_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn get_shadow(st: &AppState, tid: Uuid, did: Uuid) -> ServiceResult<ShadowResponse> {
    let shadow = shadow_repo::get_or_create_shadow(&st.db, tid, did)
        .await
        .map_err(map_db_error)?;
    Ok(to_shadow_response(did, shadow))
}

pub async fn update_shadow_desired(
    st: &AppState,
    tid: Uuid,
    did: Uuid,
    body: &UpdateShadowDesiredReq,
) -> ServiceResult<ShadowResponse> {
    body.validate()?;
    let shadow = shadow_repo::update_shadow_desired(&st.db, tid, did, &body.desired)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "update_desired",
        "device_shadow",
        &did.to_string(),
        json!({}),
    )
    .await
    .ok();
    Ok(to_shadow_response(did, shadow))
}

pub async fn update_shadow_reported(
    st: &AppState,
    tid: Uuid,
    did: Uuid,
    body: &UpdateShadowReportedReq,
) -> ServiceResult<ShadowResponse> {
    body.validate()?;
    let shadow = shadow_repo::update_shadow_reported(&st.db, tid, did, &body.reported)
        .await
        .map_err(map_db_error)?;
    Ok(to_shadow_response(did, shadow))
}

fn to_shadow_response(device_id: Uuid, shadow: DeviceShadow) -> ShadowResponse {
    let delta = compute_shadow_delta(&shadow.desired, &shadow.reported);
    ShadowResponse {
        device_id,
        reported: shadow.reported,
        desired: shadow.desired,
        delta,
        reported_at: shadow.reported_at,
        desired_at: shadow.desired_at,
        version: shadow.version,
    }
}
