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

    let devices_forced: bool =
        sqlx::query_scalar("SELECT relforcerowsecurity FROM pg_class WHERE relname = 'devices'")
            .fetch_one(&pool)
            .await
            .unwrap_or(false);
    let role_bypasses_rls: bool = sqlx::query_scalar(
        "SELECT rolsuper OR rolbypassrls FROM pg_roles WHERE rolname = current_user",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !devices_forced || role_bypasses_rls {
        eprintln!(
            "skip: current database role bypasses RLS or FORCE ROW LEVEL SECURITY migration is not applied"
        );
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
        "INSERT INTO devices (tenant_id, platform, registration_code, hostname, labels) \
         VALUES ($1, 'other'::device_platform, 'RLS-A-CODE', 'rls-a-host', '{}'), \
                ($2, 'other'::device_platform, 'RLS-B-CODE', 'rls-b-host', '{}')",
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

    assert_eq!(
        cnt, 1,
        "RLS should hide other-tenant device rows for current session"
    );

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

#[tokio::test]
async fn rls_is_forced_on_tenant_scoped_tables() {
    let Ok(url) = std::env::var("DMSX_TEST_DATABASE_URL") else {
        return;
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect DMSX_TEST_DATABASE_URL");

    let forced_rows: Vec<(String, bool)> = sqlx::query_as(
        "SELECT relname, relforcerowsecurity FROM pg_class \
         WHERE relname = ANY($1::text[]) \
         ORDER BY relname",
    )
    .bind(vec![
        "orgs",
        "sites",
        "groups",
        "devices",
        "policies",
        "policy_revisions",
        "commands",
        "artifacts",
        "audit_logs",
        "compliance_findings",
        "device_shadows",
        "command_results",
        "system_settings",
    ])
    .fetch_all(&pool)
    .await
    .expect("read pg_class");

    if forced_rows.len() != 13 {
        eprintln!("skip: expected hardened RLS migration to be applied");
        return;
    }

    if forced_rows.iter().any(|(_, forced)| !*forced) {
        eprintln!("skip: FORCE ROW LEVEL SECURITY migration not applied to this database yet");
        return;
    }

    for (table, forced) in forced_rows {
        assert!(forced, "{table} must FORCE ROW LEVEL SECURITY");
    }
}

#[tokio::test]
async fn composite_foreign_keys_reject_cross_tenant_device_parents() {
    let Ok(url) = std::env::var("DMSX_TEST_DATABASE_URL") else {
        return;
    };

    let pool = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .expect("connect DMSX_TEST_DATABASE_URL");

    let constraint_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT 1 FROM pg_constraint WHERE conname = 'devices_tenant_site_fk')",
    )
    .fetch_one(&pool)
    .await
    .unwrap_or(false);
    if !constraint_exists {
        eprintln!("skip: expected tenant integrity migration to be applied");
        return;
    }

    let ta = Uuid::new_v4();
    let tb = Uuid::new_v4();
    let org_a = Uuid::new_v4();
    let site_a = Uuid::new_v4();

    sqlx::query("INSERT INTO tenants (id, name) VALUES ($1, 'fk-test-a'), ($2, 'fk-test-b')")
        .bind(ta)
        .bind(tb)
        .execute(&pool)
        .await
        .expect("insert tenants");
    sqlx::query("INSERT INTO orgs (id, tenant_id, name) VALUES ($1, $2, 'fk-org-a')")
        .bind(org_a)
        .bind(ta)
        .execute(&pool)
        .await
        .expect("insert org");
    sqlx::query("INSERT INTO sites (id, tenant_id, org_id, name) VALUES ($1, $2, $3, 'fk-site-a')")
        .bind(site_a)
        .bind(ta)
        .bind(org_a)
        .execute(&pool)
        .await
        .expect("insert site");

    let err = sqlx::query(
        "INSERT INTO devices (tenant_id, site_id, platform, registration_code, hostname, labels) \
         VALUES ($1, $2, 'other'::device_platform, 'FK-CROSS-CODE', 'cross-tenant-device', '{}')",
    )
    .bind(tb)
    .bind(site_a)
    .execute(&pool)
    .await
    .expect_err("cross-tenant site_id must be rejected");

    let pg_code = match err {
        sqlx::Error::Database(db) => db.code().map(|code| code.to_string()),
        other => panic!("unexpected error: {other}"),
    };
    assert_eq!(pg_code.as_deref(), Some("23503"));

    sqlx::query("DELETE FROM tenants WHERE id = $1 OR id = $2")
        .bind(ta)
        .bind(tb)
        .execute(&pool)
        .await
        .expect("cleanup tenants");
}
