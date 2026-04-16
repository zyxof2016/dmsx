//! 租户 / 组织 / 站点 / 设备组 写路径编排（控制面 REST）。

use dmsx_core::{Group, Org, Site, Tenant};
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{CreateGroupReq, CreateOrgReq, CreateSiteReq, CreateTenantReq};
use crate::error::map_db_error;
use crate::repo::{audit, groups, orgs, sites, tenants};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn create_tenant(
    st: &AppState,
    ctx: &AuthContext,
    body: &CreateTenantReq,
) -> ServiceResult<Tenant> {
    body.validate()?;
    let mut tx = st.db.begin().await.map_err(map_db_error)?;
    let tenant = tenants::insert_tenant(&mut *tx, &body.name)
        .await
        .map_err(map_db_error)?;
    db_rls::apply_session_vars(&mut tx, Some(tenant.id.0), ctx.is_platform_admin())
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tenant.id.0,
        "create",
        "tenant",
        &tenant.id.0.to_string(),
        json!({ "name": &tenant.name }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(tenant)
}

pub async fn create_org(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &CreateOrgReq,
) -> ServiceResult<Org> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let org = orgs::insert_org(&mut *tx, tid, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "org",
        &org.id.0.to_string(),
        json!({ "name": &org.name }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(org)
}

pub async fn create_site(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    org_id: Uuid,
    body: &CreateSiteReq,
) -> ServiceResult<Site> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let site = sites::insert_site(&mut *tx, tid, org_id, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "site",
        &site.id.0.to_string(),
        json!({ "org_id": org_id, "name": &site.name }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(site)
}

pub async fn create_group(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    site_id: Uuid,
    body: &CreateGroupReq,
) -> ServiceResult<Group> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let group = groups::insert_group(&mut *tx, tid, site_id, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "group",
        &group.id.0.to_string(),
        json!({ "site_id": site_id, "name": &group.name }),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(group)
}
