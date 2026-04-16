use dmsx_core::{Device, DmsxError};
use serde_json::json;
use uuid::Uuid;

use crate::dto::{CreateDeviceReq, DeviceListParams, ListResponse, UpdateDeviceReq};
use crate::error::map_db_error;
use crate::repo::{audit, devices as device_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_devices(
    st: &AppState,
    tid: Uuid,
    params: &DeviceListParams,
) -> ServiceResult<ListResponse<Device>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = device_repo::list_devices(&st.db, tid, params)
        .await
        .map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

pub async fn create_device(
    st: &AppState,
    tid: Uuid,
    body: &CreateDeviceReq,
) -> ServiceResult<Device> {
    body.validate()?;
    let device = device_repo::create_device(&st.db, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "device",
        &device.id.0.to_string(),
        json!({"platform": format!("{:?}", body.platform), "hostname": &body.hostname}),
    )
    .await
    .ok();
    Ok(device)
}

pub async fn get_device(st: &AppState, tid: Uuid, did: Uuid) -> ServiceResult<Device> {
    device_repo::get_device(&st.db, tid, did)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))
}

pub async fn update_device(
    st: &AppState,
    tid: Uuid,
    did: Uuid,
    body: &UpdateDeviceReq,
) -> ServiceResult<Device> {
    body.validate()?;
    let device = device_repo::update_device(&st.db, tid, did, body)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    audit::write_audit(&st.db, tid, "update", "device", &did.to_string(), json!({}))
        .await
        .ok();
    Ok(device)
}

pub async fn delete_device(st: &AppState, tid: Uuid, did: Uuid) -> ServiceResult<()> {
    if device_repo::delete_device(&st.db, tid, did)
        .await
        .map_err(map_db_error)?
    {
        audit::write_audit(&st.db, tid, "delete", "device", &did.to_string(), json!({}))
            .await
            .ok();
        Ok(())
    } else {
        Err(DmsxError::NotFound(format!("device {did}")))
    }
}
