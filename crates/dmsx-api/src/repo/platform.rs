use chrono::{DateTime, Utc};
use sqlx::{PgConnection, Row};

use crate::dto::{AuditLog, AuditLogListParams, PlatformHealth, PlatformTenantSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlatformUsageCounts {
    pub tenant_count: i64,
    pub device_count: i64,
    pub command_count: i64,
    pub artifact_count: i64,
}

pub async fn usage_counts(conn: &mut PgConnection) -> Result<PlatformUsageCounts, sqlx::Error> {
    let row = sqlx::query(
        "
        SELECT
            (SELECT COUNT(*)::bigint FROM tenants) AS tenant_count,
            (SELECT COUNT(*)::bigint FROM devices) AS device_count,
            (SELECT COUNT(*)::bigint FROM commands) AS command_count,
            (SELECT COUNT(*)::bigint FROM artifacts) AS artifact_count",
    )
    .fetch_one(&mut *conn)
    .await?;

    Ok(PlatformUsageCounts {
        tenant_count: row.try_get("tenant_count")?,
        device_count: row.try_get("device_count")?,
        command_count: row.try_get("command_count")?,
        artifact_count: row.try_get("artifact_count")?,
    })
}

pub async fn list_platform_audit_logs(
    conn: &mut PgConnection,
    params: &AuditLogListParams,
) -> Result<(Vec<AuditLog>, i64), sqlx::Error> {
    let lim = params.limit();
    let off = params.offset();

    let count_sql = "
        SELECT COUNT(*)::bigint
        FROM audit_logs
        WHERE ($1::text IS NULL OR action = $1)
          AND ($2::text IS NULL OR resource_type = $2)";

    let total = sqlx::query_scalar::<_, i64>(count_sql)
        .bind(params.action.as_deref())
        .bind(params.resource_type.as_deref())
        .fetch_one(&mut *conn)
        .await?;

    let rows = sqlx::query(
        "
        SELECT id, actor_user_id, action, resource_type, resource_id, payload, created_at
        FROM audit_logs
        WHERE ($1::text IS NULL OR action = $1)
          AND ($2::text IS NULL OR resource_type = $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4",
    )
    .bind(params.action.as_deref())
    .bind(params.resource_type.as_deref())
    .bind(lim)
    .bind(off)
    .fetch_all(&mut *conn)
    .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let actor_user_id = row.try_get("actor_user_id")?;
        items.push(AuditLog {
            id: row.try_get("id")?,
            created_at,
            actor_user_id,
            action: row.try_get("action")?,
            resource_type: row.try_get("resource_type")?,
            resource_id: row.try_get("resource_id")?,
            payload: row.try_get("payload")?,
        });
    }

    Ok((items, total))
}

pub async fn platform_health(conn: &mut PgConnection) -> Result<PlatformHealth, sqlx::Error> {
    let counts = usage_counts(conn).await?;

    Ok(PlatformHealth {
        status: "ok".to_string(),
        tenant_count: counts.tenant_count,
        device_count: counts.device_count,
        command_count: counts.command_count,
    })
}

pub async fn tenant_summaries(conn: &mut PgConnection) -> Result<Vec<PlatformTenantSummary>, sqlx::Error> {
    let rows = sqlx::query(
        "
        SELECT t.id, t.name, t.created_at,
               COUNT(DISTINCT d.id)::bigint AS device_count,
               COUNT(DISTINCT p.id)::bigint AS policy_count,
               COUNT(DISTINCT c.id)::bigint AS command_count
        FROM tenants t
        LEFT JOIN devices d ON d.tenant_id = t.id
        LEFT JOIN policies p ON p.tenant_id = t.id
        LEFT JOIN commands c ON c.tenant_id = t.id
        GROUP BY t.id, t.name, t.created_at
        ORDER BY t.created_at DESC",
    )
    .fetch_all(conn)
    .await?;

    rows.into_iter()
        .map(|row| {
            Ok(PlatformTenantSummary {
                id: row.try_get("id")?,
                name: row.try_get("name")?,
                created_at: row.try_get("created_at")?,
                device_count: row.try_get("device_count")?,
                policy_count: row.try_get("policy_count")?,
                command_count: row.try_get("command_count")?,
            })
        })
        .collect()
}
