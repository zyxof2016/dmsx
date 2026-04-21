use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use chrono::{Duration, Utc};
use dmsx_core::{Device, DmsxError, EnrollStatus, OnlineState};
use hmac::{Hmac, Mac};
use serde_json::json;
use serde_json::Value;
use sha2::Sha256;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{
    BatchCreateDevicesReq, BatchCreateDevicesResponse, ClaimDeviceEnrollmentReq, CreateDeviceReq,
    DeviceEnrollmentBatchResponse, DeviceEnrollmentToken, DeviceListParams,
    IssueDeviceEnrollmentTokenReq, ListResponse, UpdateDeviceReq,
};
use crate::error::map_db_error;
use crate::repo::{audit, device_enrollment_batches as batch_repo, devices as device_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

type HmacSha256 = Hmac<Sha256>;

fn enroll_token_secret(st: &AppState) -> ServiceResult<&str> {
    st.enroll_token_hmac_secret.as_deref().ok_or_else(|| {
        DmsxError::Internal(
            "device enroll token signing is not configured (missing DMSX_API_ENROLL_TOKEN_HMAC_SECRET)"
                .into(),
        )
    })
}

fn sign_device_enrollment_token(payload: &Value, secret: &str) -> ServiceResult<String> {
    let payload_raw = serde_json::to_vec(payload)
        .map_err(|e| DmsxError::Internal(format!("serialize device enroll token payload: {e}")))?;
    let payload_b64 = URL_SAFE_NO_PAD.encode(payload_raw);
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| DmsxError::Internal("device enroll token hmac init".into()))?;
    mac.update(payload_b64.as_bytes());
    let sig_b64 = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    Ok(format!("v1.{payload_b64}.{sig_b64}"))
}

fn verify_device_enrollment_token(token: &str, secret: &str) -> ServiceResult<Value> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 || parts[0] != "v1" {
        return Err(DmsxError::Validation("invalid enrollment_token format".into()));
    }

    let payload_b64 = parts[1];
    let sig_b64 = parts[2];
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes())
        .map_err(|_| DmsxError::Internal("device enroll token hmac init".into()))?;
    mac.update(payload_b64.as_bytes());
    let expected_sig_b64 = URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes());
    if expected_sig_b64 != sig_b64 {
        return Err(DmsxError::Unauthorized("invalid enrollment token signature".into()));
    }

    let payload_raw = URL_SAFE_NO_PAD
        .decode(payload_b64)
        .map_err(|_| DmsxError::Validation("invalid enrollment token payload encoding".into()))?;
    let payload: Value = serde_json::from_slice(&payload_raw)
        .map_err(|_| DmsxError::Validation("invalid enrollment token payload json".into()))?;
    let exp = payload
        .get("exp")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| DmsxError::Validation("enrollment token missing exp".into()))?;
    if Utc::now().timestamp() > exp {
        return Err(DmsxError::Unauthorized("enrollment token expired".into()));
    }
    Ok(payload)
}

fn batch_response_to_json(response: &BatchCreateDevicesResponse) -> Result<Value, DmsxError> {
    serde_json::to_value(response)
        .map_err(|e| DmsxError::Internal(format!("serialize batch response: {e}")))
}

fn batch_response_from_json(
    batch_id: Uuid,
    created_at: chrono::DateTime<Utc>,
    value: Value,
) -> Result<DeviceEnrollmentBatchResponse, DmsxError> {
    let response: BatchCreateDevicesResponse = serde_json::from_value(value)
        .map_err(|e| DmsxError::Internal(format!("deserialize batch response: {e}")))?;
    Ok(DeviceEnrollmentBatchResponse {
        batch_id,
        devices: response.devices,
        enrollment_tokens: response.enrollment_tokens,
        created_at,
    })
}

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
        json!({
            "platform": format!("{:?}", body.platform),
            "hostname": &body.hostname,
            "registration_code": &device.registration_code,
        }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}

pub async fn batch_create_devices(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &BatchCreateDevicesReq,
) -> ServiceResult<BatchCreateDevicesResponse> {
    body.validate()?;
    let secret = if body.issue_enrollment_tokens() {
        Some(enroll_token_secret(st)?)
    } else {
        None
    };
    let expires_at = Utc::now() + Duration::seconds(body.ttl_seconds());

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;

    let mut devices = Vec::with_capacity(body.items.len());
    let mut enrollment_tokens = Vec::new();

    for item in &body.items {
        let device = device_repo::create_device(&mut *tx, tid, item)
            .await
            .map_err(map_db_error)?;
        audit::write_audit(
            &mut *tx,
            tid,
            "create",
            "device",
            &device.id.0.to_string(),
            json!({
                "platform": format!("{:?}", item.platform),
                "hostname": &item.hostname,
                "registration_code": &device.registration_code,
                "batch": true,
            }),
        )
        .await
        .ok();

        if let Some(secret) = secret {
            let payload = json!({
                "tenant_id": tid,
                "device_id": device.id.0,
                "registration_code": &device.registration_code,
                "exp": expires_at.timestamp(),
            });
            let token = sign_device_enrollment_token(&payload, secret)?;
            enrollment_tokens.push(DeviceEnrollmentToken {
                token,
                expires_at,
                registration_code: device.registration_code.clone(),
                device_id: device.id.0,
            });
        }

        devices.push(device);
    }

    let response = BatchCreateDevicesResponse {
        batch_id: Uuid::nil(),
        devices,
        enrollment_tokens,
    };
    let response_json = batch_response_to_json(&response)?;
    let batch = batch_repo::insert_batch(
        &mut *tx,
        tid,
        Some(&ctx.subject),
        response.devices.len() as i64,
        &response_json,
    )
    .await
    .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;

    Ok(BatchCreateDevicesResponse {
        batch_id: batch.id,
        devices: response.devices,
        enrollment_tokens: response.enrollment_tokens,
    })
}

pub async fn get_device_enrollment_batch(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    batch_id: Uuid,
) -> ServiceResult<DeviceEnrollmentBatchResponse> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let batch = batch_repo::get_batch(&mut *tx, tid, batch_id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device enrollment batch {batch_id}")))?;
    tx.commit().await.map_err(map_db_error)?;
    batch_response_from_json(batch.id, batch.created_at, batch.result)
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
    audit::write_audit(
        &mut *tx,
        tid,
        "update",
        "device",
        &did.to_string(),
        json!({
            "registration_code": &device.registration_code,
            "hostname": &device.hostname,
        }),
    )
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

pub async fn rotate_registration_code(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
) -> ServiceResult<Device> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let device = device_repo::rotate_registration_code(&mut *tx, tid, did)
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    audit::write_audit(
        &mut *tx,
        tid,
        "rotate_registration_code",
        "device",
        &did.to_string(),
        json!({"registration_code": &device.registration_code}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}

pub async fn issue_device_enrollment_token(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    did: Uuid,
    body: &IssueDeviceEnrollmentTokenReq,
) -> ServiceResult<DeviceEnrollmentToken> {
    body.validate()?;
    let secret = enroll_token_secret(st)?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let device = device_repo::get_device(&mut *tx, tid, did)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {did}")))?;
    let expires_at = Utc::now() + Duration::seconds(body.ttl_seconds());
    let payload = json!({
        "tenant_id": tid,
        "device_id": did,
        "registration_code": &device.registration_code,
        "exp": expires_at.timestamp(),
    });
    let token = sign_device_enrollment_token(&payload, secret)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "issue_enrollment_token",
        "device",
        &did.to_string(),
        json!({"registration_code": &device.registration_code, "expires_at": expires_at}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(DeviceEnrollmentToken {
        token,
        expires_at,
        registration_code: device.registration_code,
        device_id: did,
    })
}

pub async fn claim_device_with_enrollment_token(
    st: &AppState,
    tid: Uuid,
    body: &ClaimDeviceEnrollmentReq,
) -> ServiceResult<Device> {
    body.validate()?;
    let secret = enroll_token_secret(st)?;
    let payload = verify_device_enrollment_token(&body.enrollment_token, secret)?;
    let token_tenant_id = payload
        .get("tenant_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DmsxError::Validation("enrollment token missing tenant_id".into()))?
        .parse::<Uuid>()
        .map_err(|_| DmsxError::Validation("invalid enrollment token tenant_id".into()))?;
    let token_device_id = payload
        .get("device_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DmsxError::Validation("enrollment token missing device_id".into()))?
        .parse::<Uuid>()
        .map_err(|_| DmsxError::Validation("invalid enrollment token device_id".into()))?;
    let token_registration_code = payload
        .get("registration_code")
        .and_then(|v| v.as_str())
        .ok_or_else(|| DmsxError::Validation("enrollment token missing registration_code".into()))?;

    if token_tenant_id != tid {
        return Err(DmsxError::Forbidden("enrollment token tenant mismatch".into()));
    }

    let synthetic_ctx = AuthContext {
        subject: "device-enroll".into(),
        tenant_id: tid,
        roles: vec!["TenantAdmin".into()],
    };
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), &synthetic_ctx)
        .await
        .map_err(map_db_error)?;
    let existing = device_repo::get_device(&mut *tx, tid, token_device_id)
        .await
        .map_err(map_db_error)?
        .ok_or_else(|| DmsxError::NotFound(format!("device {token_device_id}")))?;
    if existing.registration_code != token_registration_code {
        return Err(DmsxError::Forbidden("enrollment token registration code mismatch".into()));
    }

    let device = device_repo::update_device(
        &mut *tx,
        tid,
        token_device_id,
        &UpdateDeviceReq {
            registration_code: None,
            hostname: body.hostname.clone(),
            os_version: body.os_version.clone(),
            agent_version: body.agent_version.clone(),
            enroll_status: Some(EnrollStatus::Active),
            online_state: Some(OnlineState::Online),
            labels: Some(body.labels.clone()),
        },
    )
    .await
    .map_err(map_db_error)?
    .ok_or_else(|| DmsxError::NotFound(format!("device {token_device_id}")))?;
    audit::write_audit(
        &mut *tx,
        tid,
        "claim_with_enrollment_token",
        "device",
        &token_device_id.to_string(),
        json!({"hostname": &device.hostname, "registration_code": &device.registration_code}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(device)
}
