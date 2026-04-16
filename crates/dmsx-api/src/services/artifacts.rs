use dmsx_core::Artifact;
use serde_json::json;
use uuid::Uuid;

use crate::dto::{ArtifactListParams, CreateArtifactReq, ListResponse};
use crate::error::map_db_error;
use crate::repo::{artifacts as artifact_repo, audit};
use crate::services::ServiceResult;
use crate::state::AppState;

pub async fn list_artifacts(
    st: &AppState,
    tid: Uuid,
    params: &ArtifactListParams,
) -> ServiceResult<ListResponse<Artifact>> {
    let lim = params.limit();
    let off = params.offset();
    let (items, total) = artifact_repo::list_artifacts(&st.db, tid, params)
        .await
        .map_err(map_db_error)?;
    Ok(ListResponse {
        items,
        total,
        limit: lim,
        offset: off,
    })
}

pub async fn create_artifact(
    st: &AppState,
    tid: Uuid,
    body: &CreateArtifactReq,
) -> ServiceResult<Artifact> {
    body.validate()?;
    let artifact = artifact_repo::create_artifact(&st.db, tid, body)
        .await
        .map_err(map_db_error)?;
    audit::write_audit(
        &st.db,
        tid,
        "create",
        "artifact",
        &artifact.id.0.to_string(),
        json!({"name": &body.name, "version": &body.version}),
    )
    .await
    .ok();
    Ok(artifact)
}
