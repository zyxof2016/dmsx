use dmsx_core::{Command, CommandResult, CommandStatus};
use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::{CommandListParams, CreateCommandReq};

const COMMAND_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::command_status IS NULL OR status = $2) \
    AND ($3::uuid IS NULL OR target_device_id = $3)";

pub async fn list_commands(
    pool: &PgPool,
    tid: Uuid,
    p: &CommandListParams,
) -> Result<(Vec<Command>, i64), sqlx::Error> {
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM commands {COMMAND_WHERE}");
    let data_sql = format!(
        "SELECT * FROM commands {COMMAND_WHERE} ORDER BY created_at DESC LIMIT $4 OFFSET $5"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(p.status)
            .bind(p.target_device_id)
            .fetch_one(pool),
        sqlx::query_as::<_, Command>(&data_sql)
            .bind(tid)
            .bind(p.status)
            .bind(p.target_device_id)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

pub async fn get_command(
    pool: &PgPool,
    tid: Uuid,
    cid: Uuid,
) -> Result<Option<Command>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM commands WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(cid)
        .fetch_optional(pool)
        .await
}

pub async fn create_command(
    pool: &PgPool,
    tid: Uuid,
    r: &CreateCommandReq,
) -> Result<Command, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO commands (tenant_id, target_device_id, payload, priority, ttl_seconds, idempotency_key) \
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING *",
    )
    .bind(tid)
    .bind(r.target_device_id)
    .bind(&r.payload)
    .bind(r.priority.unwrap_or(0i16))
    .bind(r.ttl_seconds.unwrap_or(3600i32))
    .bind(&r.idempotency_key)
    .fetch_one(pool)
    .await
}

pub async fn get_command_result(
    pool: &PgPool,
    tid: Uuid,
    cid: Uuid,
) -> Result<Option<CommandResult>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM command_results WHERE tenant_id = $1 AND command_id = $2")
        .bind(tid)
        .bind(cid)
        .fetch_optional(pool)
        .await
}

pub async fn upsert_command_result(
    pool: &PgPool,
    tid: Uuid,
    cid: Uuid,
    exit_code: Option<i32>,
    stdout: &str,
    stderr: &str,
    evidence_key: Option<&str>,
) -> Result<CommandResult, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO command_results (command_id, tenant_id, exit_code, stdout, stderr, evidence_key) \
         VALUES ($1, $2, $3, $4, $5, $6) \
         ON CONFLICT (command_id) DO UPDATE SET \
           exit_code = $3, stdout = $4, stderr = $5, evidence_key = $6, reported_at = now() \
         RETURNING *",
    )
    .bind(cid)
    .bind(tid)
    .bind(exit_code)
    .bind(stdout)
    .bind(stderr)
    .bind(evidence_key)
    .fetch_one(pool)
    .await
}

pub async fn update_command_status(
    pool: &PgPool,
    tid: Uuid,
    cid: Uuid,
    status: CommandStatus,
) -> Result<Option<Command>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE commands SET status = $3, updated_at = now() \
         WHERE tenant_id = $1 AND id = $2 RETURNING *",
    )
    .bind(tid)
    .bind(cid)
    .bind(status)
    .fetch_optional(pool)
    .await
}

pub async fn list_device_commands(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
    limit: i64,
    offset: i64,
) -> Result<(Vec<Command>, i64), sqlx::Error> {
    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(
            "SELECT COUNT(*)::bigint FROM commands WHERE tenant_id = $1 AND target_device_id = $2"
        )
        .bind(tid)
        .bind(did)
        .fetch_one(pool),
        sqlx::query_as::<_, Command>(
            "SELECT * FROM commands WHERE tenant_id = $1 AND target_device_id = $2 \
             ORDER BY created_at DESC LIMIT $3 OFFSET $4"
        )
        .bind(tid)
        .bind(did)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool),
    )?;

    Ok((items, total))
}
