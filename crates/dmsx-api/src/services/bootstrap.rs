use uuid::Uuid;

use crate::repo::tenants;
use crate::state::AppState;

pub async fn ensure_default_tenant(st: &AppState, tid: Uuid, name: &str) {
    if let Err(err) = tenants::ensure_tenant(&st.db, tid, name).await {
        tracing::warn!("failed to ensure default tenant {tid}: {err}");
    }
}
