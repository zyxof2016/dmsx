use chrono::{DateTime, Utc};
use sqlx::PgConnection;
use sqlx::Row;

use crate::dto::{SystemSetting, SystemSettingUpsertReq};

pub async fn get_global_setting(
    conn: &mut PgConnection,
    key: &str,
) -> Result<Option<SystemSetting>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT key, value, updated_at \
         FROM system_settings \
         WHERE tenant_id IS NULL AND key = $1",
    )
    .bind(key)
    .fetch_optional(conn)
    .await?;

    if let Some(row) = row {
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        Ok(Some(SystemSetting {
            key: row.try_get("key")?,
            value: row.try_get("value")?,
            updated_at,
        }))
    } else {
        Ok(None)
    }
}

pub async fn upsert_global_setting(
    conn: &mut PgConnection,
    key: &str,
    req: SystemSettingUpsertReq,
) -> Result<SystemSetting, sqlx::Error> {
    let value = req.value;

    let updated = sqlx::query(
        "UPDATE system_settings \
         SET value = $2, updated_at = now() \
         WHERE tenant_id IS NULL AND key = $1 \
         RETURNING key, value, updated_at",
    )
    .bind(key)
    .bind(value.clone())
    .fetch_optional(&mut *conn)
    .await?;

    if let Some(row) = updated {
        let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
        return Ok(SystemSetting {
            key: row.try_get("key")?,
            value: row.try_get("value")?,
            updated_at,
        });
    }

    let row = sqlx::query(
        "INSERT INTO system_settings (tenant_id, key, value) \
         VALUES (NULL, $1, $2) \
         RETURNING key, value, updated_at",
    )
    .bind(key)
    .bind(value)
    .fetch_one(&mut *conn)
    .await?;

    let updated_at: DateTime<Utc> = row.try_get("updated_at")?;
    Ok(SystemSetting {
        key: row.try_get("key")?,
        value: row.try_get("value")?,
        updated_at,
    })
}

