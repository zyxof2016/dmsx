//! S3/MinIO-backed evidence persistence for `UploadEvidence`.

use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use uuid::Uuid;

fn env_trimmed(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn env_bool(name: &str, default: bool) -> bool {
    std::env::var(name)
        .ok()
        .map(|v| v.trim().to_ascii_lowercase())
        .map(|v| matches!(v.as_str(), "1" | "true" | "yes" | "on"))
        .unwrap_or(default)
}

pub(crate) struct EvidenceStore {
    client: Client,
    bucket: String,
    key_prefix: String,
}

impl EvidenceStore {
    pub(crate) async fn from_env() -> Result<Option<Self>, String> {
        let Some(bucket) = env_trimmed("DMSX_GW_EVIDENCE_S3_BUCKET") else {
            return Ok(None);
        };

        let region = env_trimmed("DMSX_GW_EVIDENCE_S3_REGION")
            .unwrap_or_else(|| "us-east-1".to_string());
        let endpoint = env_trimmed("DMSX_GW_EVIDENCE_S3_ENDPOINT");
        let access_key = env_trimmed("DMSX_GW_EVIDENCE_S3_ACCESS_KEY");
        let secret_key = env_trimmed("DMSX_GW_EVIDENCE_S3_SECRET_KEY");
        let key_prefix = env_trimmed("DMSX_GW_EVIDENCE_S3_PREFIX")
            .unwrap_or_else(|| "evidence".to_string())
            .trim_matches('/')
            .to_string();
        let force_path_style = env_bool(
            "DMSX_GW_EVIDENCE_S3_FORCE_PATH_STYLE",
            endpoint.is_some(),
        );

        let mut loader = aws_config::defaults(BehaviorVersion::latest()).region(Region::new(region));
        match (access_key, secret_key) {
            (Some(access_key), Some(secret_key)) => {
                loader = loader.credentials_provider(Credentials::new(
                    access_key,
                    secret_key,
                    None,
                    None,
                    "dmsx-device-gw-evidence",
                ));
            }
            (None, None) => {}
            _ => {
                return Err(
                    "DMSX_GW_EVIDENCE_S3_ACCESS_KEY and DMSX_GW_EVIDENCE_S3_SECRET_KEY must be set together"
                        .to_string(),
                )
            }
        }

        let shared_config = loader.load().await;
        let mut conf = aws_sdk_s3::config::Builder::from(&shared_config);
        if let Some(endpoint) = endpoint {
            conf = conf.endpoint_url(endpoint);
        }
        conf = conf.force_path_style(force_path_style);

        Ok(Some(Self {
            client: Client::from_conf(conf.build()),
            bucket,
            key_prefix,
        }))
    }

    pub(crate) async fn put_object(
        &self,
        tenant_id: Uuid,
        device_id: Uuid,
        content_type: &str,
        body: Vec<u8>,
    ) -> Result<String, String> {
        let object_key = format!(
            "{}/{}/{}/{}",
            self.key_prefix,
            tenant_id,
            device_id,
            Uuid::new_v4()
        );

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&object_key)
            .body(ByteStream::from(body))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| format!("put evidence object: {e}"))?;

        Ok(object_key)
    }
}
