use chrono::Utc;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{TenantRbacRolesResponse, TenantRbacRolesUpsertReq};
use crate::dto::{TenantRbacMeResponse, TenantRoleBindingsResponse, TenantRoleBindingsUpsertReq};
use crate::error::map_db_error;
use crate::repo::{audit, system_settings as settings_repo};
use crate::services::ServiceResult;
use crate::state::AppState;
use crate::tenant_rbac::{
    build_custom_roles_value, build_role_bindings_value, load_custom_roles_from_conn,
    load_role_bindings_from_conn, normalize_custom_roles, normalize_role_bindings,
    TENANT_RBAC_ROLES_KEY, TENANT_ROLE_BINDINGS_KEY,
};

pub async fn get_tenant_rbac_roles(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
) -> ServiceResult<TenantRbacRolesResponse> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;

    let response = if let Some((custom_roles, updated_at)) =
        load_custom_roles_from_conn(&mut tx, tenant_id).await.map_err(map_db_error)?
    {
        TenantRbacRolesResponse {
            key: TENANT_RBAC_ROLES_KEY.to_string(),
            custom_roles,
            updated_at: Some(updated_at),
        }
    } else {
        TenantRbacRolesResponse {
            key: TENANT_RBAC_ROLES_KEY.to_string(),
            custom_roles: Vec::new(),
            updated_at: None,
        }
    };

    tx.commit().await.map_err(map_db_error)?;
    Ok(response)
}

pub async fn upsert_tenant_rbac_roles(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
    req: TenantRbacRolesUpsertReq,
) -> ServiceResult<TenantRbacRolesResponse> {
    let custom_roles = normalize_custom_roles(req)
        .map_err(dmsx_core::DmsxError::Validation)?;

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;

    let setting = settings_repo::upsert_tenant_setting(
        &mut tx,
        tenant_id,
        TENANT_RBAC_ROLES_KEY,
        crate::dto::SystemSettingUpsertReq {
            value: build_custom_roles_value(&custom_roles),
        },
    )
    .await
    .map_err(map_db_error)?;

    audit::write_audit(
        &mut tx,
        tenant_id,
        "update",
        "tenant_rbac_roles",
        TENANT_RBAC_ROLES_KEY,
        json!({ "custom_role_count": custom_roles.len(), "updated_at": Utc::now() }),
    )
    .await
    .ok();

    tx.commit().await.map_err(map_db_error)?;

    st.tenant_custom_roles
        .write()
        .await
        .insert(tenant_id, custom_roles.clone());

    Ok(TenantRbacRolesResponse {
        key: setting.key,
        custom_roles,
        updated_at: Some(setting.updated_at),
    })
}

pub async fn get_tenant_role_bindings(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
) -> ServiceResult<TenantRoleBindingsResponse> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;

    let response = if let Some((bindings, updated_at)) =
        load_role_bindings_from_conn(&mut tx, tenant_id).await.map_err(map_db_error)?
    {
        TenantRoleBindingsResponse {
            key: TENANT_ROLE_BINDINGS_KEY.to_string(),
            bindings,
            updated_at: Some(updated_at),
        }
    } else {
        TenantRoleBindingsResponse {
            key: TENANT_ROLE_BINDINGS_KEY.to_string(),
            bindings: Vec::new(),
            updated_at: None,
        }
    };

    tx.commit().await.map_err(map_db_error)?;
    Ok(response)
}

pub async fn upsert_tenant_role_bindings(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
    req: TenantRoleBindingsUpsertReq,
) -> ServiceResult<TenantRoleBindingsResponse> {
    let bindings = normalize_role_bindings(req).map_err(dmsx_core::DmsxError::Validation)?;

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tenant_id), ctx)
        .await
        .map_err(map_db_error)?;

    let setting = settings_repo::upsert_tenant_setting(
        &mut tx,
        tenant_id,
        TENANT_ROLE_BINDINGS_KEY,
        crate::dto::SystemSettingUpsertReq {
            value: build_role_bindings_value(&bindings),
        },
    )
    .await
    .map_err(map_db_error)?;

    audit::write_audit(
        &mut tx,
        tenant_id,
        "update",
        "tenant_role_bindings",
        TENANT_ROLE_BINDINGS_KEY,
        json!({ "binding_count": bindings.len(), "updated_at": Utc::now() }),
    )
    .await
    .ok();

    tx.commit().await.map_err(map_db_error)?;

    st.tenant_role_bindings
        .write()
        .await
        .insert(tenant_id, bindings.clone());

    Ok(TenantRoleBindingsResponse {
        key: setting.key,
        bindings,
        updated_at: Some(setting.updated_at),
    })
}

pub async fn get_tenant_rbac_me(
    st: &AppState,
    ctx: &AuthContext,
    tenant_id: Uuid,
) -> ServiceResult<TenantRbacMeResponse> {
    let binding_roles = st
        .tenant_role_bindings
        .read()
        .await
        .get(&tenant_id)
        .and_then(|bindings| bindings.iter().find(|binding| binding.subject == ctx.subject))
        .map(|binding| binding.roles.clone())
        .unwrap_or_default();

    let effective_roles = if binding_roles.is_empty() {
        ctx.roles.clone()
    } else {
        binding_roles.clone()
    };

    Ok(TenantRbacMeResponse {
        tenant_id,
        subject: ctx.subject.clone(),
        jwt_roles: ctx.roles.clone(),
        binding_roles,
        effective_roles,
    })
}
