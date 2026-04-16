use dmsx_core::Artifact;
use sqlx::PgPool;
use uuid::Uuid;

use crate::dto::{ArtifactListParams, CreateArtifactReq};

const ARTIFACT_WHERE: &str = "\
    WHERE tenant_id = $1 \
    AND ($2::text IS NULL OR name ILIKE '%' || $2 || '%')";

pub async fn list_artifacts(
    pool: &PgPool,
    tid: Uuid,
    p: &ArtifactListParams,
) -> Result<(Vec<Artifact>, i64), sqlx::Error> {
    let search = p.search_term();
    let lim = p.limit();
    let off = p.offset();

    let count_sql = format!("SELECT COUNT(*)::bigint FROM artifacts {ARTIFACT_WHERE}");
    let data_sql = format!(
        "SELECT * FROM artifacts {ARTIFACT_WHERE} ORDER BY created_at DESC LIMIT $3 OFFSET $4"
    );

    let (total, items) = tokio::try_join!(
        sqlx::query_scalar::<_, i64>(&count_sql)
            .bind(tid)
            .bind(search)
            .fetch_one(pool),
        sqlx::query_as::<_, Artifact>(&data_sql)
            .bind(tid)
            .bind(search)
            .bind(lim)
            .bind(off)
            .fetch_all(pool),
    )?;

    Ok((items, total))
}

pub async fn create_artifact(
    pool: &PgPool,
    tid: Uuid,
    r: &CreateArtifactReq,
) -> Result<Artifact, sqlx::Error> {
    sqlx::query_as(
        "INSERT INTO artifacts (tenant_id, name, version, sha256, channel, object_key, metadata) \
         VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *",
    )
    .bind(tid)
    .bind(&r.name)
    .bind(&r.version)
    .bind(&r.sha256)
    .bind(r.channel.as_deref().unwrap_or("stable"))
    .bind(&r.object_key)
    .bind(r.metadata.as_ref().unwrap_or(&serde_json::json!({})))
    .fetch_one(pool)
    .await
}
