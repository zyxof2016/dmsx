//! 可选集成验证：需要已应用仓库内迁移（含 `005_rls_tenant_isolation.sql`）的 Postgres。
//! 未设置 `DMSX_TEST_DATABASE_URL` 时本文件中的测试会立即返回（视为跳过）。

use sqlx::postgres::PgPoolOptions;
use uuid::Uuid;

#[tokio::test]
async fn rls_hides_other_tenant_devices_under_transaction_local_settings() {
    let Ok(url) = std::env::var("DMSX_TEST_DATABASE_URL") else {
        return;
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect DMSX_TEST_DATABASE_URL");

    let helpers_ok: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_proc p \
         JOIN pg_namespace n ON n.oid = p.pronamespace \
         WHERE n.nspname = 'dmsx' AND p.proname = 'current_tenant_id')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);

    if !helpers_ok {
        eprintln!("skip: dmsx.current_tenant_id() missing (migrations not applied?)");
        return;
    }

    let ta = Uuid::new_v4();
    let tb = Uuid::new_v4();

    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'rls-test-a'), ($2, 'rls-test-b')")
        .bind(ta)
        .bind(tb)
        .execute(&pool)
        .await
        .expect("insert tenants");

    sqlx::query(
        "INSERT INTO devices (tenant_id, platform, hostname, labels) \
         VALUES ($1, 'other'::device_platform, 'rls-a-host', '{}'), \
                ($2, 'other'::device_platform, 'rls-b-host', '{}')",
    )
    .bind(ta)
    .bind(tb)
    .execute(&pool)
    .await
    .expect("insert devices");

    let mut tx = pool.begin().await.expect("begin");
    sqlx::query(
        "SELECT set_config('dmsx.tenant_id', $1, true), set_config('dmsx.is_platform_admin', $2, true)",
    )
    .bind(ta.to_string())
    .bind("false")
    .execute(&mut *tx)
    .await
    .expect("set_config");

    let cnt: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM devices")
        .fetch_one(&mut *tx)
        .await
        .expect("count");

    assert_eq!(cnt, 1, "RLS should hide other-tenant device rows for current session");

    let peek_b: Option<Uuid> = sqlx::query_scalar("SELECT id FROM devices WHERE tenant_id = $1")
        .bind(tb)
        .fetch_optional(&mut *tx)
        .await
        .expect("select tenant B");

    assert!(
        peek_b.is_none(),
        "must not observe tenant B device id while session is scoped to tenant A"
    );

    tx.rollback().await.ok();

    sqlx::query("DELETE FROM devices WHERE tenant_id = $1 OR tenant_id = $2")
        .bind(ta)
        .bind(tb)
        .execute(&pool)
        .await
        .expect("cleanup devices");
    sqlx::query("DELETE FROM tenants WHERE id = $1 OR id = $2")
        .bind(ta)
        .bind(tb)
        .execute(&pool)
        .await
        .expect("cleanup tenants");
}
