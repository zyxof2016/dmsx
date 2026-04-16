use dmsx_core::Tenant;
use sqlx::PgConnection;
use uuid::Uuid;

pub async fn ensure_tenant(conn: &mut PgConnection, tid: Uuid, name: &str) -> Result<Tenant, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO tenants (id, name) VALUES ($1, $2) \
         ON CONFLICT (id) DO UPDATE SET name = EXCLUDED.name \
         RETURNING *",
    )
    .bind(tid)
    .bind(name)
    .fetch_one(conn)
    .await
}

/// 新建租户（服务端生成 `id`）。需由上层做 **PlatformAdmin** 等权限约束。
pub async fn insert_tenant(conn: &mut PgConnection, name: &str) -> Result<Tenant, sqlx::Error> {
    sqlx::query_as::<_, Tenant>("INSERT INTO tenants (name) VALUES ($1) RETURNING *")
        .bind(name)
        .fetch_one(conn)
        .await
}
