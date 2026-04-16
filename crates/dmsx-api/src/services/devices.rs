use dmsx_core::{Device, DmsxError};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{CreateDeviceReq, DeviceListParams, ListResponse, UpdateDeviceReq};
use crate::error::map_db_error;
use crate::repo::{audit, devices as device_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_devices(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &DeviceListParams,
) -> ServiceResult<ListResponse<Device>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = device_repo::list_devices(&mut *tx, tid, params)
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

pub async fn create_device(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &CreateDeviceReq,
) -> ServiceResult<Device> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let device = device_repo::create_device(&mut *tx, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "device",
        &device.id.0.to_string(),
        json!({"platform": format!("{:?}", body.platform), "hostname": &body.hostname}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}

pub async fn get_device(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
) -> ServiceResult<Device> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let device = device_repo::get_device(&mut *tx, tid, did)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}

pub async fn update_device(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
    body: &UpdateDeviceReq,
) -> ServiceResult<Device> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let device = device_repo::update_device(&mut *tx, tid, did, body)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    audit::write_audit(&mut *tx, tid, "update", "device", &did.to_string(), json!({}))
        .await
        .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}

pub async fn delete_device(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
) -> ServiceResult<()> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    if device_repo::delete_device(&mut *tx, tid, did)
        .await
        .map_err(map_db_error)?
    {
        audit::write_audit(&mut *tx, tid, "delete", "device", &did.to_string(), json!({}))
            .await
            .ok();
        tx.commit().await.map_err(map_db_error)?;
        Ok(())
    } else {
        tx.commit().await.map_err(map_db_error)?;
        Err(DmsxError::NotFound(format!("device {did}")))
    }
}
