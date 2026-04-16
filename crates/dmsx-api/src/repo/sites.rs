use dmsx_core::Site;
use sqlx::PgConnection;
use uuid::Uuid;

/// 在租户下创建站点；`org_id` 必须属于同一 `tenant_id`，否则不插入任何行。
pub async fn insert_site(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    org_id: Uuid,
    name: &str,
) -> Result<Site, sqlx::Error> {
    sqlx::query_as::<_, Site>(
        "INSERT INTO sites (tenant_id, org_id, name) \
         SELECT $1, o.id, $3 FROM orgs o \
         WHERE o.id = $2 AND o.tenant_id = $1 \
         RETURNING *",
    )
    .bind(tenant_id)
    .bind(org_id)
    .bind(name)
    .fetch_one(conn)
    .await
}
