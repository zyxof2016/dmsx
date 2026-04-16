use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::{CountBucket, DashboardStats};
use crate::query_types::{CountBucketRow, StatsRow};

pub async fn get_stats(pool: &PgPool, tid: Uuid) -> Result<DashboardStats, sqlx::Error> {
    let (base, platforms, cmd_statuses, sev_counts) = tokio::try_join!(
        sqlx::query_as::<_, StatsRow>(
            "SELECT \
               (SELECT COUNT(*) FROM devices WHERE tenant_id = $1)::bigint AS device_total, \
               (SELECT COUNT(*) FROM devices WHERE tenant_id = $1 AND online_state = 'online')::bigint AS device_online, \
               (SELECT COUNT(*) FROM policies WHERE tenant_id = $1)::bigint AS policy_count, \
               (SELECT COUNT(*) FROM commands WHERE tenant_id = $1 AND status IN ('queued','delivered','running'))::bigint AS command_pending, \
               (SELECT COUNT(*) FROM compliance_findings WHERE tenant_id = $1 AND status = 'open')::bigint AS finding_open"
        )
        .bind(tid)
        .fetch_one(pool),
        sqlx::query_as::<_, CountBucketRow>(
            "SELECT platform::text AS label, COUNT(*)::bigint AS count \
             FROM devices WHERE tenant_id = $1 GROUP BY platform ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
        sqlx::query_as::<_, CountBucketRow>(
            "SELECT status::text AS label, COUNT(*)::bigint AS count \
             FROM commands WHERE tenant_id = $1 GROUP BY status ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
        sqlx::query_as::<_, CountBucketRow>(
            "SELECT severity::text AS label, COUNT(*)::bigint AS count \
             FROM compliance_findings WHERE tenant_id = $1 AND status = 'open' \
             GROUP BY severity ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
    )?;

    Ok(DashboardStats {
        device_total: base.device_total,
        device_online: base.device_online,
        policy_count: base.policy_count,
        command_pending: base.command_pending,
        finding_open: base.finding_open,
        platforms: platforms.into_iter().map(Into::into).collect(),
        command_statuses: cmd_statuses.into_iter().map(Into::into).collect(),
        finding_severities: sev_counts.into_iter().map(Into::into).collect(),
    })
}

impl From<CountBucketRow> for CountBucket {
    fn from(value: CountBucketRow) -> Self {
        Self {
            label: value.label,
            count: value.count,
        }
    }
}
