use dmsx_core::ComplianceFinding;
use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::FindingListParams;

const FINDING_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR (title ILIKE '%' || $2 || '%' OR rule_id ILIKE '%' || $2 || '%')) \
    AND ($3::finding_severity IS NULL OR severity = $3) \
    AND ($4::finding_status IS NULL OR status = $4)";

pub async fn list_findings(
    pool: &PgPool,
    tid: Uuid,
    p: &FindingListParams,
) -> Result<(Vec<ComplianceFinding>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM compliance_findings {FINDING_WHERE}");
    let data_sql = format!(
        "SELECT * FROM compliance_findings {FINDING_WHERE} ORDER BY detected_at DESC LIMIT $5 OFFSET $6"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .bind(p.severity)
            .bind(p.status)
            .fetch_one(pool),
        sqlx::query_as::<_, ComplianceFinding>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(p.severity)
            .bind(p.status)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}
