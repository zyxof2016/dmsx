use dmsx_core::Artifact;
use serde_json::json;
use uuid::Uuid;

use crate::auth::AuthContext;
use crate::db_rls;
use crate::dto::{ArtifactListParams, CreateArtifactReq, ListResponse};
use crate::error::map_db_error;
use crate::repo::{artifacts as artifact_repo, audit};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_artifacts(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    params: &ArtifactListParams,
) -> ServiceResult<ListResponse<Artifact>> {
    let lim = params.limit();
    let off = params.offset();
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let (items, total) = artifact_repo::list_artifacts(&mut *tx, tid, params)
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

pub async fn create_artifact(
    st: &AppState,
    ctx: &AuthContext,
    tid: Uuid,
    body: &CreateArtifactReq,
) -> ServiceResult<Artifact> {
    body.validate()?;
    let mut tx = db_rls::begin_rls_tx(&st.db, Some(tid), ctx)
        .await
        .map_err(map_db_error)?;
    let artifact = artifact_repo::create_artifact(&mut *tx, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &mut *tx,
        tid,
        "create",
        "artifact",
        &artifact.id.0.to_string(),
        json!({"name": &body.name, "version": &body.version}),
    )
    .await
    .ok();
    tx.commit().await.map_err(map_db_error)?;
    Ok(artifact)
}
