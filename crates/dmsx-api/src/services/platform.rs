use crate::auth::AuthContext;
use crate::dto::{AuditLog, AuditLogListParams, ListResponse, PlatformHealth, PlatformQuota, PlatformTenantSummary};
use crate::error::map_db_error;
use crate::repo::platform;
use crate::repo::platform::PlatformUsageCounts;
use crate::services::ServiceResult;
use crate::state::AppState;

fn quota_limit_from_env(name: &str, default: i64) -> i64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .map(|v| v.max(1))
        .unwrap_or(default)
}

fn build_platform_quotas(counts: PlatformUsageCounts) -> ListResponse<PlatformQuota> {
    let items = vec![
        PlatformQuota {
            key: "tenants".into(),
            limit: quota_limit_from_env("DMSX_API_PLATFORM_TENANT_LIMIT", 1_000),
            used: counts.tenant_count,
            unit: "count".into(),
        },
        PlatformQuota {
            key: "devices".into(),
            limit: quota_limit_from_env("DMSX_API_PLATFORM_DEVICE_LIMIT", 10_000),
            used: counts.device_count,
            unit: "count".into(),
        },
        PlatformQuota {
            key: "commands".into(),
            limit: quota_limit_from_env("DMSX_API_PLATFORM_COMMAND_LIMIT", 100_000),
            used: counts.command_count,
            unit: "count".into(),
        },
        PlatformQuota {
            key: "artifacts".into(),
            limit: quota_limit_from_env("DMSX_API_PLATFORM_ARTIFACT_LIMIT", 10_000),
            used: counts.artifact_count,
            unit: "count".into(),
        },
    ];

    ListResponse {
        total: items.len() as i64,
        limit: items.len() as i64,
        offset: 0,
        items,
    }
}

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

pub async fn platform_quotas(
    st: &AppState,
    _ctx: &AuthContext,
) -> ServiceResult<ListResponse<PlatformQuota>> {
    let mut conn = st.db.acquire().await.map_err(map_db_error)?;
    let counts = platform::usage_counts(&mut conn).await.map_err(map_db_error)?;
    Ok(build_platform_quotas(counts))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quota_limit_from_env_uses_default_for_invalid_values() {
        std::env::remove_var("DMSX_API_PLATFORM_DEVICE_LIMIT");
        assert_eq!(quota_limit_from_env("DMSX_API_PLATFORM_DEVICE_LIMIT", 42), 42);

        std::env::set_var("DMSX_API_PLATFORM_DEVICE_LIMIT", "bad");
        assert_eq!(quota_limit_from_env("DMSX_API_PLATFORM_DEVICE_LIMIT", 42), 42);
        std::env::remove_var("DMSX_API_PLATFORM_DEVICE_LIMIT");
    }

    #[test]
    fn quota_limit_from_env_clamps_to_positive_values() {
        std::env::set_var("DMSX_API_PLATFORM_DEVICE_LIMIT", "0");
        assert_eq!(quota_limit_from_env("DMSX_API_PLATFORM_DEVICE_LIMIT", 42), 1);

        std::env::set_var("DMSX_API_PLATFORM_DEVICE_LIMIT", "128");
        assert_eq!(quota_limit_from_env("DMSX_API_PLATFORM_DEVICE_LIMIT", 42), 128);
        std::env::remove_var("DMSX_API_PLATFORM_DEVICE_LIMIT");
    }

    #[test]
    fn build_platform_quotas_uses_counts_and_defaults() {
        let response = build_platform_quotas(PlatformUsageCounts {
            tenant_count: 3,
            device_count: 12,
            command_count: 34,
            artifact_count: 5,
        });

        assert_eq!(response.total, 4);
        assert_eq!(response.items[0].key, "tenants");
        assert_eq!(response.items[0].used, 3);
        assert_eq!(response.items[1].key, "devices");
        assert_eq!(response.items[1].used, 12);
        assert_eq!(response.items[2].key, "commands");
        assert_eq!(response.items[2].used, 34);
        assert_eq!(response.items[3].key, "artifacts");
        assert_eq!(response.items[3].used, 5);
    }
}
