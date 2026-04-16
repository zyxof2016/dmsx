use dmsx_core::Tenant;
use sqlx::PgPool;
use uuid::Uuid;

pub async fn ensure_tenant(pool: &PgPool, tid: Uuid, name: &str) -> Result<Tenant, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO tenants (id, name) VALUES ($1, $2) \
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name \
         RETURNING *",
    )
    .bind(tid)
    .bind(name)
    .fetch_one(pool)
    .await
}
