use uuid::Uuid;

use crate::dto::DashboardStats;
use crate::error::map_db_error;
use crate::repo::stats as stats_repo;
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn get_stats(st: &AppState, tid: Uuid) -> ServiceResult<DashboardStats> {
    stats_repo::get_stats(&st.db, tid)
        .await
        .map_err(map_db_error)
}
