use dmsx_core::Org;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn insert_org(pool: &PgPool, tenant_id: Uuid, name: &str) -> Result<Org, sqlx::Error> {
    sqlx::query_as::<_, Org>(
        "INSERT INTO orgs (tenant_id, name) VALUES ($1, $2) RETURNING *",
    )
    .bind(tenant_id)
    .bind(name)
    .fetch_one(pool)
    .await
}
