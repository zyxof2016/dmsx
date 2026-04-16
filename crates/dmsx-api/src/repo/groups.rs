use dmsx_core::Group;
use sqlx::PgPool;
use uuid::Uuid;

/// 在租户下创建设备组；`site_id` 必须属于同一 `tenant_id`。
pub async fn insert_group(
    pool: &PgPool,
    tenant_id: Uuid,
    site_id: Uuid,
    name: &str,
) -> Result<Group, sqlx::Error> {
    sqlx::query_as::<_, Group>(
        "INSERT INTO groups (tenant_id, site_id, name) \
         SELECT s.tenant_id, s.id, $3 FROM sites s \
         WHERE s.id = $2 AND s.tenant_id = $1 \
         RETURNING *",
    )
    .bind(tenant_id)
    .bind(site_id)
    .bind(name)
    .fetch_one(pool)
    .await
}
