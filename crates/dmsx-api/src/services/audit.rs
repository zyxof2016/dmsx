use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{AuditLog, AuditLogListParams, ListResponse};
use crate::error::map_db_error;
use crate::repo::audit;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_audit_logs(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &AuditLogListParams,
) -> ServiceResult<ListResponse<AuditLog>> {
    let lim = params.limit();
    let off = params.offset();

    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;

    let (items, total) = audit::list_audit_logs(&mut *tx, tid, params)
        .await
        .map_err(map_db_error)?;

    tx.commit().await.map_err(map_db_error)?;

    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

