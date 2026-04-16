//! 租户 / 组织 / 站点 / 设备组 写路径编排（控制面 REST）。

use dmsx_core::{Group, Org, Site, Tenant};
use serde_json::json;
use uuid::Uuid;

use crate::dto::{CreateGroupReq, CreateOrgReq, CreateSiteReq, CreateTenantReq};
use crate::error::map_db_error;
use crate::repo::{audit, groups, orgs, sites, tenants};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn create_tenant(st: &AppState, body: &CreateTenantReq) -> ServiceResult<Tenant> {
    body.validate()?;
    let tenant = tenants::insert_tenant(&st.db, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tenant.id.0,
        "create",
        "tenant",
        &tenant.id.0.to_string(),
        json!({ "name": &tenant.name }),
    )
    .await
    .ok();
    Ok(tenant)
}

pub async fn create_org(st: &AppState, tid: Uuid, body: &CreateOrgReq) -> ServiceResult<Org> {
    body.validate()?;
    let org = orgs::insert_org(&st.db, tid, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "org",
        &org.id.0.to_string(),
        json!({ "name": &org.name }),
    )
    .await
    .ok();
    Ok(org)
}

pub async fn create_site(
    st: &AppState,
    tid: Uuid,
    org_id: Uuid,
    body: &CreateSiteReq,
) -> ServiceResult<Site> {
    body.validate()?;
    let site = sites::insert_site(&st.db, tid, org_id, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "site",
        &site.id.0.to_string(),
        json!({ "org_id": org_id, "name": &site.name }),
    )
    .await
    .ok();
    Ok(site)
}

pub async fn create_group(
    st: &AppState,
    tid: Uuid,
    site_id: Uuid,
    body: &CreateGroupReq,
) -> ServiceResult<Group> {
    body.validate()?;
    let group = groups::insert_group(&st.db, tid, site_id, &body.name)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "group",
        &group.id.0.to_string(),
        json!({ "site_id": site_id, "name": &group.name }),
    )
    .await
    .ok();
    Ok(group)
}
