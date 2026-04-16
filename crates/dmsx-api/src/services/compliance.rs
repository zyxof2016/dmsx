use dmsx_core::ComplianceFinding;
use uuid::Uuid;

use crate::dto::{FindingListParams, ListResponse};
use crate::error::map_db_error;
use crate::repo::compliance as compliance_repo;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_findings(
    st: &AppState,
    tid: Uuid,
    params: &FindingListParams,
) -> ServiceResult<ListResponse<ComplianceFinding>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = compliance_repo::list_findings(&st.db, tid, params)
        .await
        .map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}
