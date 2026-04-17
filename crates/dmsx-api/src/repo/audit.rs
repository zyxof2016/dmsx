use chrono::{DateTime, SecondsFormat, Utc};
use reqwest::Client;
use sqlx::PgConnection;
use sqlx::Row;
use uuid::Uuid;

use crate::dto::{AuditLog, AuditLogListParams};

use serde::Serialize;

pub async fn write_audit(
    conn: &mut PgConnection,
    tid: Uuid,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    let audit_id = Uuid::new_v4();
    let created_at = Utc::now();
    let actor_user_id: Option<Uuid> = None;

    // Authoritative audit write: PostgreSQL transaction.
    sqlx::query(
        "INSERT INTO audit_logs \
         (id, tenant_id, actor_user_id, action, resource_type, resource_id, payload, created_at) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(audit_id)
    .bind(tid)
    .bind(actor_user_id)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(payload.clone())
    .bind(created_at)
    .execute(conn)
    .await?;

    // Optional async write to ClickHouse (analytics).
    // Kept behind env gate to avoid breaking unit tests / dev setups.
    let ch_url = std::env::var("DMSX_CLICKHOUSE_HTTP_URL")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    if ch_url.is_some() {
        let ch_url = ch_url.unwrap();
        let payload_str = payload.to_string();
        let created_at_str = created_at.to_rfc3339_opts(SecondsFormat::Millis, true);
        let action_s = action.to_string();
        let resource_type_s = resource_type.to_string();
        let resource_id_s = resource_id.to_string();
        let tenant_id = tid;
        let audit_id = audit_id;

        tokio::spawn(async move {
            let insert_sql = concat!(
                "INSERT INTO audit_events ",
                "(id, tenant_id, actor_user_id, action, resource_type, resource_id, payload, created_at) ",
                "FORMAT JSONEachRow"
            );

            #[derive(Serialize)]
            struct AuditEventRow {
                id: Uuid,
                tenant_id: Uuid,
                actor_user_id: Option<Uuid>,
                action: String,
                resource_type: String,
                resource_id: String,
                payload: String,
                created_at: String,
            }

            let row = AuditEventRow {
                id: audit_id,
                tenant_id,
                actor_user_id: None,
                action: action_s,
                resource_type: resource_type_s,
                resource_id: resource_id_s,
                payload: payload_str,
                created_at: created_at_str,
            };

            let row_json = match serde_json::to_string(&row) {
                Ok(s) => s,
                Err(e) => {
                    tracing::warn!(error = %e, "clickhouse audit row json serialize failed");
                    return;
                }
            };

            // ClickHouse HTTP accepts the SQL query as request body for POST.
            // Body format: "<SQL>\n<JSONEachRow row>\n"
            let body = format!("{insert_sql}\n{row_json}\n");

            let client = Client::new();
            let mut req = client.post(&ch_url).body(body);

            if let (Ok(u), Ok(p)) = (
                std::env::var("DMSX_CLICKHOUSE_HTTP_USER"),
                std::env::var("DMSX_CLICKHOUSE_HTTP_PASSWORD"),
            ) {
                if !u.trim().is_empty() && !p.trim().is_empty() {
                    req = req.basic_auth(u, Some(p));
                }
            }

            if let Err(e) = req.send().await {
                tracing::warn!(error = %e, "clickhouse audit_events write failed");
            }
        });
    }
    Ok(())
}

pub async fn list_audit_logs(
    conn: &mut PgConnection,
    tid: Uuid,
    p: &AuditLogListParams,
) -> Result<(Vec<AuditLog>, i64), sqlx::Error> {
    let lim = p.limit();
    let off = p.offset();

    let count_sql = "\
        SELECT COUNT(*)::bigint \
        FROM audit_logs \
        WHERE tenant_id = $1 \
          AND ($2::text IS NULL OR action = $2) \
          AND ($3::text IS NULL OR resource_type = $3)";

    let total = sqlx::query_scalar::<_, i64>(count_sql)
        .bind(tid)
        .bind(p.action.as_deref())
        .bind(p.resource_type.as_deref())
        .fetch_one(&mut *conn)
        .await?;

    let data_sql = "\
        SELECT \
            id, actor_user_id, action, resource_type, resource_id, payload, created_at \
        FROM audit_logs \
        WHERE tenant_id = $1 \
          AND ($2::text IS NULL OR action = $2) \
          AND ($3::text IS NULL OR resource_type = $3) \
        ORDER BY created_at DESC \
        LIMIT $4 OFFSET $5";

    let rows = sqlx::query(data_sql)
        .bind(tid)
        .bind(p.action.as_deref())
        .bind(p.resource_type.as_deref())
        .bind(lim)
        .bind(off)
        .fetch_all(&mut *conn)
        .await?;

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let created_at: DateTime<Utc> = row.try_get("created_at")?;
        let actor_user_id: Option<Uuid> = row.try_get("actor_user_id")?;
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
