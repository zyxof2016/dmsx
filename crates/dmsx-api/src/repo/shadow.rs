use dmsx_core::DeviceShadow;
use sqlx::PgConnection;
use uuid::Uuid;

pub async fn get_or_create_shadow(
    conn: &mut PgConnection,
    tid: Uuid,
    did: Uuid,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query(
        "INSERT INTO device_shadows (device_id, tenant_id) \
         SELECT d.id, d.tenant_id FROM devices d \
         WHERE d.id = $1 AND d.tenant_id = $2 \
         ON CONFLICT (device_id) DO NOTHING",
    )
    .bind(did)
    .bind(tid)
    .execute(&mut *conn)
    .await?;

    sqlx::query_as("SELECT * FROM device_shadows WHERE device_id = $1 AND tenant_id = $2")
        .bind(did)
        .bind(tid)
        .fetch_one(&mut *conn)
        .await
}

pub async fn update_shadow_desired(
    conn: &mut PgConnection,
    tid: Uuid,
    did: Uuid,
    desired: &serde_json::Value,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO device_shadows (device_id, tenant_id, desired, desired_at, version) \
         SELECT d.id, d.tenant_id, $3, now(), 1 FROM devices d \
         WHERE d.id = $1 AND d.tenant_id = $2 \
         ON CONFLICT (device_id) DO UPDATE SET \
            desired = $3, desired_at = now(), version = device_shadows.version + 1 \
         RETURNING *",
    )
    .bind(did)
    .bind(tid)
    .bind(desired)
    .fetch_one(&mut *conn)
    .await
}

pub async fn update_shadow_reported(
    conn: &mut PgConnection,
    tid: Uuid,
    did: Uuid,
    reported: &serde_json::Value,
) -> Result<DeviceShadow, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO device_shadows (device_id, tenant_id, reported, reported_at, version) \
         SELECT d.id, d.tenant_id, $3, now(), 1 FROM devices d \
         WHERE d.id = $1 AND d.tenant_id = $2 \
         ON CONFLICT (device_id) DO UPDATE SET \
            reported = $3, reported_at = now(), version = device_shadows.version + 1 \
         RETURNING *",
    )
    .bind(did)
    .bind(tid)
    .bind(reported)
    .fetch_one(&mut *conn)
    .await
}
