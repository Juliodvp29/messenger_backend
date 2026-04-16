use aws_sdk_s3::Client;
use aws_sdk_s3::config::{Builder as S3ConfigBuilder, Credentials, Region};
use aws_sdk_s3::presigning::PresigningConfig;
use aws_sdk_s3::types::{
    BucketLifecycleConfiguration, ExpirationStatus, LifecycleExpiration, LifecycleRule,
};
use shared::config::S3Config;
use std::time::Duration;

use crate::services::error::ServiceError;

#[derive(Clone)]
pub struct S3StorageService {
    client: Client,
    bucket: String,
    endpoint: String,
}

impl S3StorageService {
    pub fn new(config: &S3Config) -> Self {
        let credentials = Credentials::new(
            config.access_key_id.clone(),
            config.secret_access_key.clone(),
            None,
            None,
            "static",
        );

        let s3_conf = S3ConfigBuilder::new()
            .region(Region::new(config.region.clone()))
            .endpoint_url(config.endpoint.clone())
            .credentials_provider(credentials)
            .force_path_style(true)
            .behavior_version_latest()
            .build();

        Self {
            client: Client::from_conf(s3_conf),
            bucket: config.bucket.clone(),
            endpoint: config.endpoint.clone(),
        }
    }

    pub async fn presign_put_url(
        &self,
        key: &str,
        content_type: &str,
        ttl_seconds: u64,
    ) -> Result<String, ServiceError> {
        let presign_config = PresigningConfig::expires_in(Duration::from_secs(ttl_seconds))
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let req = self
            .client
            .put_object()
            .bucket(&self.bucket)
            .key(key)
            .content_type(content_type)
            .presigned(presign_config)
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        Ok(req.uri().to_string())
    }

    pub async fn head_object(&self, key: &str) -> Result<(), ServiceError> {
        self.client
            .head_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| ServiceError::Internal(e.to_string()))?;
        Ok(())
    }

    pub fn bucket_object_url(&self, key: &str) -> String {
        format!(
            "{}/{}/{}",
            self.endpoint.trim_end_matches('/'),
            self.bucket,
            key
        )
    }

    pub async fn apply_lifecycle_rules(&self) -> Result<(), ServiceError> {
        let story_rule = LifecycleRule::builder()
            .id("ExpireStoriesAfter24H")
            .prefix("attachments/stories/")
            .status(ExpirationStatus::Enabled)
            .expiration(LifecycleExpiration::builder().days(1).build())
            .build()
            .map_err(|e| {
                ServiceError::Internal(format!("Failed to build lifecycle rule: {}", e))
            })?;

        let config = BucketLifecycleConfiguration::builder()
            .rules(story_rule)
            .build()
            .map_err(|e| {
                ServiceError::Internal(format!("Failed to build lifecycle config: {}", e))
            })?;

        self.client
            .put_bucket_lifecycle_configuration()
            .bucket(&self.bucket)
            .lifecycle_configuration(config)
            .send()
            .await
            .map_err(|e| {
                tracing::error!("Failed to apply S3 lifecycle rules: {:?}", e);
                ServiceError::Internal(format!("Failed to apply lifecycle rules: {}", e))
            })?;

        tracing::info!(
            "S3 lifecycle rules applied successfully to bucket {}",
            self.bucket
        );
        Ok(())
    }
}
