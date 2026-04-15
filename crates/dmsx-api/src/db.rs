use dmsx_core::*;
use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::*;

// ---------------------------------------------------------------------------
// Dashboard stats (multiple queries assembled)
// ---------------------------------------------------------------------------

pub async fn get_stats(pool: &PgPool, tid: Uuid) -> Result<DashboardStats, sqlx::Error> {
    let (base, platforms, cmd_statuses, sev_counts) = tokio::try_join!(
        sqlx::query_as::<_, StatsRow>(
            "SELECT \
               (SELECT COUNT(*) FROM devices WHERE tenant_id = $1)::bigint AS device_total, \
               (SELECT COUNT(*) FROM devices WHERE tenant_id = $1 AND online_state = 'online')::bigint AS device_online, \
               (SELECT COUNT(*) FROM policies WHERE tenant_id = $1)::bigint AS policy_count, \
               (SELECT COUNT(*) FROM commands WHERE tenant_id = $1 AND status IN ('queued','delivered','running'))::bigint AS command_pending, \
               (SELECT COUNT(*) FROM compliance_findings WHERE tenant_id = $1 AND status = 'open')::bigint AS finding_open"
        )
        .bind(tid)
        .fetch_one(pool),
        sqlx::query_as::<_, CountBucket>(
            "SELECT platform::text AS label, COUNT(*)::bigint AS count \
             FROM devices WHERE tenant_id = $1 GROUP BY platform ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
        sqlx::query_as::<_, CountBucket>(
            "SELECT status::text AS label, COUNT(*)::bigint AS count \
             FROM commands WHERE tenant_id = $1 GROUP BY status ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
        sqlx::query_as::<_, CountBucket>(
            "SELECT severity::text AS label, COUNT(*)::bigint AS count \
             FROM compliance_findings WHERE tenant_id = $1 AND status = 'open' \
             GROUP BY severity ORDER BY count DESC"
        )
        .bind(tid)
        .fetch_all(pool),
    )?;

    Ok(DashboardStats {
        device_total: base.device_total,
        device_online: base.device_online,
        policy_count: base.policy_count,
        command_pending: base.command_pending,
        finding_open: base.finding_open,
        platforms,
        command_statuses: cmd_statuses,
        finding_severities: sev_counts,
    })
}

// ---------------------------------------------------------------------------
// Devices (paginated + filtered)
// ---------------------------------------------------------------------------

const DEVICE_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR hostname ILIKE '%' || $2 || '%') \
    AND ($3::device_platform IS NULL OR platform = $3) \
    AND ($4::enroll_status IS NULL OR enroll_status = $4) \
    AND ($5::online_state IS NULL OR online_state = $5)";

pub async fn list_devices(
    pool: &PgPool,
    tid: Uuid,
    p: &DeviceListParams,
) -> Result<(Vec<Device>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM devices {DEVICE_WHERE}");
    let data_sql = format!(
        "SELECT * FROM devices {DEVICE_WHERE} ORDER BY created_at DESC LIMIT $6 OFFSET $7"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .bind(p.platform)
            .bind(p.enroll_status)
            .bind(p.online_state)
            .fetch_one(pool),
        sqlx::query_as::<_, Device>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(p.platform)
            .bind(p.enroll_status)
            .bind(p.online_state)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

pub async fn get_device(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM devices WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(did)
        .fetch_optional(pool)
        .await
}

pub async fn create_device(
    pool: &PgPool,
    tid: Uuid,
    r: &CreateDeviceReq,
) -> Result<Device, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO devices (tenant_id, platform, hostname, os_version, agent_version, \
         site_id, primary_group_id, labels) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    )
    .bind(tid)
    .bind(r.platform)
    .bind(&r.hostname)
    .bind(&r.os_version)
    .bind(&r.agent_version)
    .bind(r.site_id)
    .bind(r.primary_group_id)
    .bind(&r.labels)
    .fetch_one(pool)
    .await
}

pub async fn update_device(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
    r: &UpdateDeviceReq,
) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as(
        "UPDATE devices SET \
         hostname       = COALESCE($3, hostname), \
         os_version     = COALESCE($4, os_version), \
         agent_version  = COALESCE($5, agent_version), \
         enroll_status  = COALESCE($6, enroll_status), \
         online_state   = COALESCE($7, online_state), \
         labels         = COALESCE($8, labels), \
         updated_at     = now() \
         WHERE tenant_id = $1 AND id = $2 RETURNING *",
    )
    .bind(tid)
    .bind(did)
    .bind(&r.hostname)
    .bind(&r.os_version)
    .bind(&r.agent_version)
    .bind(r.enroll_status)
    .bind(r.online_state)
    .bind(&r.labels)
    .fetch_optional(pool)
    .await
}

pub async fn delete_device(pool: &PgPool, tid: Uuid, did: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM devices WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(did)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

// ---------------------------------------------------------------------------
// Policies (paginated + filtered)
// ---------------------------------------------------------------------------

const POLICY_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%') \
    AND ($3::policy_scope_kind IS NULL OR scope_kind = $3)";

pub async fn list_policies(
    pool: &PgPool,
    tid: Uuid,
    p: &PolicyListParams,
) -> Result<(Vec<Policy>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM policies {POLICY_WHERE}");
    let data_sql = format!(
        "SELECT * FROM policies {POLICY_WHERE} ORDER BY created_at DESC LIMIT $4 OFFSET $5"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .bind(p.scope_kind)
            .fetch_one(pool),
        sqlx::query_as::<_, Policy>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(p.scope_kind)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

pub async fn get_policy(
    pool: &PgPool,
    tid: Uuid,
    pid: Uuid,
) -> Result<Option<Policy>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM policies WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(pid)
        .fetch_optional(pool)
        .await
}

pub async fn create_policy(
    pool: &PgPool,
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
    .fetch_one(pool)
    .await
}

pub async fn update_policy(
    pool: &PgPool,
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
    .fetch_optional(pool)
    .await
}

pub async fn delete_policy(pool: &PgPool, tid: Uuid, pid: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM policies WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(pid)
        .execute(pool)
        .await?;
    Ok(res.rows_affected() > 0)
}

pub async fn publish_policy(
    pool: &PgPool,
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
    .fetch_one(pool)
    .await
}

// ---------------------------------------------------------------------------
// Commands (paginated + filtered)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Artifacts (paginated)
// ---------------------------------------------------------------------------

const ARTIFACT_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%')";

pub async fn list_artifacts(
    pool: &PgPool,
    tid: Uuid,
    p: &ArtifactListParams,
) -> Result<(Vec<Artifact>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM artifacts {ARTIFACT_WHERE}");
    let data_sql = format!(
        "SELECT * FROM artifacts {ARTIFACT_WHERE} ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .fetch_one(pool),
        sqlx::query_as::<_, Artifact>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

pub async fn create_artifact(
    pool: &PgPool,
    tid: Uuid,
    r: &CreateArtifactReq,
) -> Result<Artifact, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO artifacts (tenant_id, name, version, sha256, channel, object_key, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(tid)
    .bind(&r.name)
    .bind(&r.version)
    .bind(&r.sha256)
    .bind(r.channel.as_deref().unwrap_or("stable"))
    .bind(&r.object_key)
    .bind(r.metadata.as_ref().unwrap_or(&serde_json::json!({})))
    .fetch_one(pool)
    .await
}

// ---------------------------------------------------------------------------
// Compliance Findings (paginated + filtered)
// ---------------------------------------------------------------------------

const FINDING_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR (title ILIKE '%' || $2 || '%' OR rule_id ILIKE '%' || $2 || '%')) \
    AND ($3::finding_severity IS NULL OR severity = $3) \
    AND ($4::finding_status IS NULL OR status = $4)";

pub async fn list_findings(
    pool: &PgPool,
    tid: Uuid,
    p: &FindingListParams,
) -> Result<(Vec<ComplianceFinding>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM compliance_findings {FINDING_WHERE}");
    let data_sql = format!(
        "SELECT * FROM compliance_findings {FINDING_WHERE} ORDER BY detected_at DESC LIMIT $5 OFFSET $6"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .bind(p.severity)
            .bind(p.status)
            .fetch_one(pool),
        sqlx::query_as::<_, ComplianceFinding>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(p.severity)
            .bind(p.status)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

// ---------------------------------------------------------------------------
// Device Shadow
// ---------------------------------------------------------------------------

pub async fn get_or_create_shadow(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query(
        "INSERT INTO device_shadows (device_id, tenant_id) VALUES ($1, $2) \
         ON CONFLICT (device_id) DO NOTHING",
    )
    .bind(did)
    .bind(tid)
    .execute(pool)
    .await?;

    sqlx::query_as(
        "SELECT * FROM device_shadows WHERE device_id = $1 AND tenant_id = $2",
    )
    .bind(did)
    .bind(tid)
    .fetch_one(pool)
    .await
}

pub async fn update_shadow_desired(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
    desired: &serde_json::Value,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO device_shadows (device_id, tenant_id, desired, desired_at, version) \
         VALUES ($1, $2, $3, now(), 1) \
         ON CONFLICT (device_id) DO UPDATE SET \
           desired = $3, desired_at = now(), version = device_shadows.version + 1 \
         RETURNING *",
    )
    .bind(did)
    .bind(tid)
    .bind(desired)
    .fetch_one(pool)
    .await
}

pub async fn update_shadow_reported(
    pool: &PgPool,
    tid: Uuid,
    did: Uuid,
    reported: &serde_json::Value,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO device_shadows (device_id, tenant_id, reported, reported_at, version) \
         VALUES ($1, $2, $3, now(), 1) \
         ON CONFLICT (device_id) DO UPDATE SET \
           reported = $3, reported_at = now(), version = device_shadows.version + 1 \
         RETURNING *",
    )
    .bind(did)
    .bind(tid)
    .bind(reported)
    .fetch_one(pool)
    .await
}

// ---------------------------------------------------------------------------
// Command results + status
// ---------------------------------------------------------------------------

pub async fn get_command_result(
    pool: &PgPool,
    tid: Uuid,
    cid: Uuid,
) -> Result<Option<CommandResult>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM command_results WHERE tenant_id = $1 AND command_id = $2",
    )
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

// ---------------------------------------------------------------------------
// Device commands (filtered by device)
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Audit log
// ---------------------------------------------------------------------------

pub async fn write_audit(
    pool: &PgPool,
    tid: Uuid,
    action: &str,
    resource_type: &str,
    resource_id: &str,
    payload: serde_json::Value,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "INSERT INTO audit_logs (tenant_id, action, resource_type, resource_id, payload) \
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(tid)
    .bind(action)
    .bind(resource_type)
    .bind(resource_id)
    .bind(payload)
    .execute(pool)
    .await?;
    Ok(())
}

// ---------------------------------------------------------------------------
// Tenants (ensure seed)
// ---------------------------------------------------------------------------

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
