use crate::auth::AuthContext;
use crate::dto::{AuditLog, AuditLogListParams, ListResponse, PlatformHealth, PlatformQuota, PlatformTenantSummary};
use crate::error::map_db_error;
use crate::repo::platform;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_tenants(
    st: &AppState,
    _ctx: &AuthContext,
) -> ServiceResult<Vec<PlatformTenantSummary>> {
    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    platform::tenant_summaries(&mut conn).await.map_err(map_db_error)
}

pub async fn list_platform_audit_logs(
    st: &AppState,
    _ctx: &AuthContext,
    params: &AuditLogListParams,
) -> ServiceResult<ListResponse<AuditLog>> {
    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    let (items, total) = platform::list_platform_audit_logs(&mut conn, params)
        .await
        .map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: params.limit(),
        offset: params.offset(),
    })
}

pub async fn platform_health(
    st: &AppState,
    _ctx: &AuthContext,
) -> ServiceResult<PlatformHealth> {
    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    platform::platform_health(&mut conn).await.map_err(map_db_error)
}

pub fn platform_quotas() -> ListResponse<PlatformQuota> {
    platform::platform_quotas()
}
