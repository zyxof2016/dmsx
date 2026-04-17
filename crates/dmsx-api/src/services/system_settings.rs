use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{SystemSetting, SystemSettingUpsertReq};
use crate::error::map_db_error;
use crate::repo::{audit, system_settings as settings_repo};
use crate::services::ServiceResult;
use crate::state::AppState;

// Default tenant used in dev seed: 00000000-0000-0000-0000-000000000001
fn dev_default_tenant_id() -> Uuid {
    Uuid::from_u128(1)
}

pub async fn get_global_setting(
    st: &AppState,
    ctx: &AuthContext,
    key: &str,
) -> ServiceResult<Option<SystemSetting>> {
    let mut tx = db_rls::begin_rls_tx(&st.db, None, ctx)
        .await
        .map_err(map_db_error)?;

    let setting = settings_repo::get_global_setting(&mut *tx, key)
        .await
        .map_err(map_db_error)?;

    tx.commit().await.map_err(map_db_error)?;
    Ok(setting)
}

pub async fn upsert_global_setting(
    st: &AppState,
    ctx: &AuthContext,
    key: &str,
    req: SystemSettingUpsertReq,
) -> ServiceResult<SystemSetting> {
    let mut tx = db_rls::begin_rls_tx(&st.db, None, ctx)
        .await
        .map_err(map_db_error)?;

    let setting = settings_repo::upsert_global_setting(&mut *tx, key, req)
        .await
        .map_err(map_db_error)?;

    // Audit logs require a non-NULL tenant_id (FK). In `disabled` auth mode the
    // active tenant may be `Uuid::nil()`, so we fall back to the seeded dev tenant.
    let audit_tid = if ctx.tenant_id.is_nil() {
        dev_default_tenant_id()
    } else {
        ctx.tenant_id
    };

    audit::write_audit(
        &mut *tx,
        audit_tid,
        "update",
        "system_setting",
        key,
        json!({}),
    )
    .await
    .ok();

    tx.commit().await.map_err(map_db_error)?;
    Ok(setting)
}

