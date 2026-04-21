use dmsx_core::DeviceEnrollmentBatch;
use sqlx::PgConnection;
use uuid::Uuid;

pub async fn insert_batch(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    actor_subject: Option<&str>,
    item_count: i64,
    result: &serde_json::Value,
) -> Result<DeviceEnrollmentBatch, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO device_enrollment_batches (tenant_id, actor_subject, item_count, result) \
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(tenant_id)
    .bind(actor_subject)
    .bind(item_count)
    .bind(result)
    .fetch_one(&mut *conn)
    .await
}

pub async fn get_batch(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    batch_id: Uuid,
) -> Result<Option<DeviceEnrollmentBatch>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM device_enrollment_batches WHERE tenant_id = $1 AND id = $2",
    )
    .bind(tenant_id)
    .bind(batch_id)
    .fetch_optional(&mut *conn)
    .await
}

pub async fn list_batches(
    conn: &mut PgConnection,
    tenant_id: Uuid,
    limit: i64,
    offset: i64,
) -> Result<Vec<DeviceEnrollmentBatch>, sqlx::Error> {
    sqlx::query_as(
        "SELECT * FROM device_enrollment_batches WHERE tenant_id = $1 ORDER BY created_at DESC LIMIT $2 OFFSET $3",
    )
    .bind(tenant_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&mut *conn)
    .await
}
