use dmsx_core::ComplianceFinding;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{FindingListParams, ListResponse};
use crate::error::map_db_error;
use crate::repo::compliance as compliance_repo;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_findings(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &FindingListParams,
) -> ServiceResult<ListResponse<ComplianceFinding>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = compliance_repo::list_findings(&mut *tx, tid, params)
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
