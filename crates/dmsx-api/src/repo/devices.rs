use dmsx_core::Device;
use sqlx::PgConnection;
use uuid::Uuid;

use crate::dto::{CreateDeviceReq, DeviceListParams, UpdateDeviceReq};

const DEVICE_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR hostname ILIKE '%' || $2 || '%') \
    AND ($3::device_platform IS NULL OR platform = $3) \
    AND ($4::enroll_status IS NULL OR enroll_status = $4) \
    AND ($5::online_state IS NULL OR online_state = $5)";

pub async fn list_devices(
    conn: &mut PgConnection,
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

    let total = sqlx::query_scalar::<_, i64>(&count_sql)
        .bind(tid)
        .bind(search)
        .bind(p.platform)
        .bind(p.enroll_status)
        .bind(p.online_state)
        .fetch_one(&mut *conn)
        .await?;

    let items = sqlx::query_as::<_, Device>(&data_sql)
        .bind(tid)
        .bind(search)
        .bind(p.platform)
        .bind(p.enroll_status)
        .bind(p.online_state)
        .bind(lim)
        .bind(off)
        .fetch_all(&mut *conn)
        .await?;

    Ok((items, total))
}

pub async fn get_device(
    conn: &mut PgConnection,
    tid: Uuid,
    did: Uuid,
) -> Result<Option<Device>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM devices WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(did)
        .fetch_optional(&mut *conn)
        .await
}

pub async fn create_device(
    conn: &mut PgConnection,
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
    .fetch_one(&mut *conn)
    .await
}

pub async fn update_device(
    conn: &mut PgConnection,
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
    .fetch_optional(&mut *conn)
    .await
}

pub async fn delete_device(conn: &mut PgConnection, tid: Uuid, did: Uuid) -> Result<bool, sqlx::Error> {
    let res = sqlx::query("DELETE FROM devices WHERE tenant_id = $1 AND id = $2")
        .bind(tid)
        .bind(did)
        .execute(&mut *conn)
        .await?;
    Ok(res.rows_affected() > 0)
}
