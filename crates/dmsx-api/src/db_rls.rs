//! Postgres session variables used by row-level security policies (see `migrations/005_rls_tenant_isolation.sql`).
//! `set_config(..., true)` is transaction-local; callers must run queries on the same [`Transaction`].

use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::auth::AuthContext;

pub async fn apply_session_vars(
    tx: &mut Transaction<'_, Postgres>,
    tenant_id: Option<Uuid>,
    is_platform_admin: bool,
) -> Result<(), sqlx::Error> {
    let tenant_str = tenant_id.map(|u| u.to_string()).unwrap_or_default();
    let admin_str = if is_platform_admin { "true" } else { "false" };
    sqlx::query(
        "SELECT set_config('dmsx.tenant_id', $1, true), set_config('dmsx.is_platform_admin', $2, true)",
    )
    .bind(tenant_str)
    .bind(admin_str)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

pub async fn begin_rls_tx<'a>(
    pool: &'a PgPool,
    tenant_id: Option<Uuid>,
    ctx: &AuthContext,
) -> Result<Transaction<'a, Postgres>, sqlx::Error> {
    let mut tx = pool.begin().await?;
    apply_session_vars(&mut tx, tenant_id, ctx.is_platform_admin()).await?;
    Ok(tx)
}
