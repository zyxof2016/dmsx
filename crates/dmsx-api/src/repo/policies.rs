use dmsx_core::{Policy, PolicyRevision};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dto::{CreatePolicyReq, PolicyListParams, UpdatePolicyReq};

const POLICY_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%') \
    AND ($3::policy_scope_kind IS NULL OR scope_kind = $3)";

pub async fn list_policies(
    conn: &mut PgConnection,
    tid: Uuid,
    p: &PolicyListParams,
) -> Result<(Vec<Policy>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM policies {POLICY_WHERE}");
    let data_sql =
        format!("SELECT * FROM policies {POLICY_WHERE} ORDER BY created_at DESC LIMIT $4 OFFSET $5");

    let total = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(tid)
        .bind(search)
        .bind(p.scope_kind)
        .fetch_one(&mut *conn)
        .await?;

    let items = sqlx::query_as::<_, Policy>(&data_sql)
        .bind(tid)
        .bind(search)
        .bind(p.scope_kind)
        .bind(lim)
        .bind(off)
        .fetch_all(&mut *conn)
        .await?;

    Ok((items, total))
}

pub async fn get_policy(
    conn: &mut PgConnection,
    tid: Uuid,
    pid: Uuid,
) -> Result<Option<Policy>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM policies WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(pid)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn create_policy(
    conn: &mut PgConnection,
    tid: Uuid,
    r: &CreatePolicyReq,
) -> Result<Policy, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO policies (tenant_id, name, description, scope_kind) \
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(tid)
    .bind(&r.name)
    .bind(&r.description)
    .bind(r.scope_kind)
    .fetch_one(&mut *conn)
    .await
}

pub async fn update_policy(
    conn: &mut PgConnection,
    tid: Uuid,
    pid: Uuid,
    r: &UpdatePolicyReq,
) -> Result<Option<Policy>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE policies SET \
         name        = COALESCE($3, name), \
         description = COALESCE($4, description), \
         scope_kind  = COALESCE($5, scope_kind), \
         updated_at  = now() \
         WHERE tenant_id = $1 AND id = $2 RETURNING *",
    )
    .bind(tid)
    .bind(pid)
    .bind(&r.name)
    .bind(&r.description)
    .bind(r.scope_kind)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn delete_policy(conn: &mut PgConnection, tid: Uuid, pid: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM policies WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(pid)
        .execute(&mut *conn)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn publish_policy(
    conn: &mut PgConnection,
    tid: Uuid,
    pid: Uuid,
    spec: serde_json::Value,
) -> Result<PolicyRevision, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO policy_revisions (tenant_id, policy_id, version, spec) \
         VALUES ($1, $2, \
           COALESCE((SELECT MAX(version) FROM policy_revisions WHERE policy_id = $2), 0) + 1, \
           $3) \
         RETURNING *",
    )
    .bind(tid)
    .bind(pid)
    .bind(spec)
    .fetch_one(&mut *conn)
    .await
}
