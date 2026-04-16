use uuid::Uuid;

use crate::repo::tenants;
use crate::state::AppState;

pub async fn ensure_default_tenant(st: &AppState, tid: Uuid, name: &str) {
    let mut conn = match st.db.acquire().await {
        Ok(c) => c,
        Err(err) => {
            tracing::warn!("failed to acquire db connection for default tenant {tid}: {err}");
            return;
        }
    };
    if let Err(err) = tenants::ensure_tenant(&mut conn, tid, name).await {
        tracing::warn!("failed to ensure default tenant {tid}: {err}");
    }
}
