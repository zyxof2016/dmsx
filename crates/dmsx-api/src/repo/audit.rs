use sqlx::PgPool;
use uuid::Uuid;

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
