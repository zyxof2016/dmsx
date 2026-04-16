//! 嵌入式 `sqlx::migrate!` 启动执行；兼容 Postgres 曾通过 `docker-entrypoint-initdb.d` 执行过同套 SQL、
//! 但未写入 `_sqlx_migrations` 的开发库（避免 `relation "tenants" already exists`）。

use sqlx::migrate::{MigrateError, Migrator};
use sqlx::PgPool;

fn migrate_err_is_duplicate_ddl(e: &MigrateError) -> bool {
    match e {
        MigrateError::ExecuteMigration(src, _) => src
            .as_database_error()
            .and_then(|db| db.code())
            .is_some_and(|c| c == "42P07"),
        _ => false,
    }
}

async fn tenants_table_exists(pool: &PgPool) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (
            SELECT 1 FROM information_schema.tables
            WHERE table_schema = 'public' AND table_name = 'tenants'
        )",
    )
    .fetch_one(pool)
    .await
}

async fn applied_success_count(pool: &PgPool) -> Result<i64, sqlx::Error> {
    sqlx::query_scalar::<_, i64>("SELECT COUNT(*)::bigint FROM _sqlx_migrations WHERE success = true")
        .fetch_one(pool)
        .await
}

/// 在确认 `public.tenants` 已存在且 **无任何成功迁移记录** 时，按嵌入式 migrator 写入 `_sqlx_migrations`，
/// 使后续 `Migrator::run` 只做校验并补跑缺失版本。
async fn backfill_sqlx_migrations_after_initdb(
    pool: &PgPool,
    migrator: &Migrator,
) -> Result<(), sqlx::Error> {
    if !tenants_table_exists(pool).await? {
        return Ok(());
    }
    if applied_success_count(pool).await? > 0 {
        return Ok(());
    }

    tracing::warn!(
        "backfilling _sqlx_migrations (schema appears pre-created, e.g. docker-entrypoint-initdb.d); \
         do not use this path if migrations were edited out-of-band"
    );

    for m in migrator.iter() {
        if !m.migration_type.is_up_migration() {
            continue;
        }
        sqlx::query(
            r#"INSERT INTO _sqlx_migrations (version, description, success, checksum, execution_time)
               VALUES ($1, $2, true, $3, 0)
               ON CONFLICT (version) DO NOTHING"#,
        )
        .bind(m.version)
        .bind(m.description.as_ref())
        .bind(m.checksum.as_ref())
        .execute(pool)
        .await?;
    }
    Ok(())
}

pub async fn run(pool: &PgPool) {
    let migrator = sqlx::migrate!("../../migrations");
    match migrator.run(pool).await {
        Ok(()) => {}
        Err(e) if migrate_err_is_duplicate_ddl(&e) => {
            tracing::warn!(error = %e, "embedded migration failed with duplicate DDL");
            if let Err(e2) = backfill_sqlx_migrations_after_initdb(pool, &migrator).await {
                panic!("failed to backfill _sqlx_migrations after duplicate DDL: {e2}");
            }
            // The first `run()` attempt may have acquired a `pg_advisory_lock` and then failed,
            // leaving the lock held by a pooled connection (session-level lock). Re-running with
            // locking enabled can deadlock in-process. For this compatibility path, disable locking
            // for the second run: the goal is to reconcile `_sqlx_migrations` with an already-
            // initialized dev schema, not to coordinate concurrent migrators.
            let migrator_no_lock = Migrator {
                migrations: migrator.migrations.clone(),
                ignore_missing: migrator.ignore_missing,
                locking: false,
                no_tx: migrator.no_tx,
            };
            migrator_no_lock
                .run(pool)
                .await
                .expect("failed to run migrations after _sqlx_migrations backfill");
        }
        Err(e) => panic!("failed to run migrations: {e}"),
    }
}
