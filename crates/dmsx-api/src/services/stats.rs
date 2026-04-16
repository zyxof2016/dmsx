use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::DashboardStats;
use crate::error::map_db_error;
use crate::repo::stats as stats_repo;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn get_stats(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
) -> ServiceResult<DashboardStats> {
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let stats = stats_repo::get_stats(&mut *tx, tid)
        .await
        .map_err(map_db_error)?;
    tx.commit().await.map_err(map_db_error)?;
    Ok(stats)
}
