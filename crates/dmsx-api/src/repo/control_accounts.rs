use serde_json::Value;
use sqlx::{PgConnection, Row};
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ControlAccountRecord {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
    pub display_name: String,
    pub platform_roles: Value,
    pub default_tenant_id: Option<Uuid>,
    pub last_tenant_id: Option<Uuid>,
    pub is_active: bool,
}

#[derive(Debug, Clone)]
pub struct ControlAccountTenantRecord {
    pub tenant_id: Uuid,
    pub roles: Value,
}

pub async fn find_by_username(
    conn: &mut PgConnection,
    username: &str,
) -> Result<Option<ControlAccountRecord>, sqlx::Error> {
    let row = sqlx::query(
        "SELECT id, username, password_hash, display_name, platform_roles, default_tenant_id, last_tenant_id, is_active \
         FROM control_accounts WHERE username = $1",
    )
    .bind(username)
    .fetch_optional(conn)
    .await?;

    row.map(map_account_row).transpose()
}

pub async fn list_tenants_for_account(
    conn: &mut PgConnection,
    account_id: Uuid,
) -> Result<Vec<ControlAccountTenantRecord>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT tenant_id, roles FROM control_account_tenants WHERE account_id = $1 ORDER BY tenant_id",
    )
    .bind(account_id)
    .fetch_all(conn)
    .await?;

    rows.into_iter().map(map_tenant_row).collect()
}

pub async fn touch_last_tenant(
    conn: &mut PgConnection,
    account_id: Uuid,
    tenant_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        "UPDATE control_accounts SET last_tenant_id = $2, updated_at = now() WHERE id = $1",
    )
    .bind(account_id)
    .bind(tenant_id)
    .execute(conn)
    .await?;
    Ok(())
}

pub async fn upsert_account(
    conn: &mut PgConnection,
    username: &str,
    password_hash: &str,
    display_name: &str,
    platform_roles: Value,
    default_tenant_id: Option<Uuid>,
    last_tenant_id: Option<Uuid>,
) -> Result<Uuid, sqlx::Error> {
    let row = sqlx::query(
        "INSERT INTO control_accounts (username, password_hash, display_name, platform_roles, default_tenant_id, last_tenant_id, is_active) \
         VALUES ($1, $2, $3, $4, $5, $6, TRUE) \
         ON CONFLICT (username) DO UPDATE SET password_hash = EXCLUDED.password_hash, display_name = EXCLUDED.display_name, platform_roles = EXCLUDED.platform_roles, default_tenant_id = EXCLUDED.default_tenant_id, last_tenant_id = EXCLUDED.last_tenant_id, is_active = TRUE, updated_at = now() \
         RETURNING id",
    )
    .bind(username)
    .bind(password_hash)
    .bind(display_name)
    .bind(platform_roles)
    .bind(default_tenant_id)
    .bind(last_tenant_id)
    .fetch_one(conn)
    .await?;

    row.try_get("id")
}

pub async fn replace_account_tenants(
    conn: &mut PgConnection,
    account_id: Uuid,
    entries: &[(Uuid, Value)],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM control_account_tenants WHERE account_id = $1")
        .bind(account_id)
        .execute(&mut *conn)
        .await?;

    for (tenant_id, roles) in entries {
        sqlx::query(
            "INSERT INTO control_account_tenants (account_id, tenant_id, roles) VALUES ($1, $2, $3)",
        )
        .bind(account_id)
        .bind(*tenant_id)
        .bind(roles)
        .execute(&mut *conn)
        .await?;
    }

    Ok(())
}

fn map_account_row(row: sqlx::postgres::PgRow) -> Result<ControlAccountRecord, sqlx::Error> {
    Ok(ControlAccountRecord {
        id: row.try_get("id")?,
        username: row.try_get("username")?,
        password_hash: row.try_get("password_hash")?,
        display_name: row.try_get("display_name")?,
        platform_roles: row.try_get("platform_roles")?,
        default_tenant_id: row.try_get("default_tenant_id")?,
        last_tenant_id: row.try_get("last_tenant_id")?,
        is_active: row.try_get("is_active")?,
    })
}

fn map_tenant_row(row: sqlx::postgres::PgRow) -> Result<ControlAccountTenantRecord, sqlx::Error> {
    Ok(ControlAccountTenantRecord {
        tenant_id: row.try_get("tenant_id")?,
        roles: row.try_get("roles")?,
    })
}
