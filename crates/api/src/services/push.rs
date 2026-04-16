use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use shared::config::PushConfig;
use sqlx::PgPool;
use std::time::Duration;
use tracing::{error, info};
use uuid::Uuid;

pub const PUSH_QUEUE_KEY: &str = "push:queue";
pub const PRESENCE_KEY_PREFIX: &str = "presence:";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushNotificationJob {
    pub user_id: Uuid,
    pub notification_type: String,
    pub payload: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PushToken {
    pub token: String,
    pub device_type: DeviceType,
}

#[derive(Debug, sqlx::FromRow)]
pub struct PushTokenRow {
    pub id: Uuid,
    pub push_token: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DeviceType {
    Android,
    Ios,
    Web,
}

impl DeviceType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Android => "android",
            Self::Ios => "ios",
            Self::Web => "web",
        }
    }
}

impl From<&str> for DeviceType {
    fn from(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "android" => Self::Android,
            "ios" => Self::Ios,
            "web" => Self::Web,
            _ => Self::Android,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmMessage {
    pub to: String,
    pub data: serde_json::Value,
    pub notification: Option<FcmNotification>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FcmNotification {
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsMessage {
    pub aps: ApnsPayload,
    pub data: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsPayload {
    pub alert: ApnsAlert,
    pub sound: Option<String>,
    pub badge: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApnsAlert {
    pub title: String,
    pub body: String,
}

pub async fn enqueue_push_notification(
    redis: &mut redis::aio::ConnectionManager,
    job: PushNotificationJob,
) -> Result<(), shared::error::DomainError> {
    let payload = serde_json::to_string(&job).map_err(|e| {
        shared::error::DomainError::Internal(format!("Failed to serialize job: {}", e))
    })?;

    let _: () = redis.lpush(PUSH_QUEUE_KEY, payload).await.map_err(|e| {
        shared::error::DomainError::Internal(format!("Failed to queue push: {}", e))
    })?;

    Ok(())
}

pub async fn push_notification_worker(
    mut redis: redis::aio::ConnectionManager,
    pool: PgPool,
    _config: PushConfig,
) {
    let _client = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
        .unwrap_or_default();

    loop {
        let pop_result: Result<Option<(String, String)>, redis::RedisError> =
            redis.brpop(PUSH_QUEUE_KEY, 1.0).await;
        match pop_result {
            Ok(Some((_, payload_str))) => {
                let job: PushNotificationJob = match serde_json::from_str(&payload_str) {
                    Ok(j) => j,
                    Err(e) => {
                        error!("Failed to deserialize push job: {}", e);
                        continue;
                    }
                };

                let tokens: Vec<PushTokenRow> = match sqlx::query_as::<_, PushTokenRow>(
                    r#"
                    SELECT id, push_token, device_type::text as device_type
                    FROM user_sessions
                    WHERE user_id = $1 AND push_token IS NOT NULL
                    "#,
                )
                .bind(job.user_id)
                .fetch_all(&pool)
                .await
                {
                    Ok(rows) => rows,
                    Err(e) => {
                        error!(
                            "DB error fetching push tokens for user {}: {}",
                            job.user_id, e
                        );
                        continue;
                    }
                };

                for token_info in tokens {
                    let session_id = token_info.id;
                    let _token = match token_info.push_token {
                        Some(t) => t,
                        None => continue,
                    };

                    let device_type =
                        DeviceType::from(token_info.device_type.as_deref().unwrap_or("android"));
                    info!(
                        "Sending push to session {} (type: {:?}) for user_id: {}",
                        session_id, device_type, job.user_id
                    );
                    // TODO: Actual HTTP call to FCM/APNs using reqwest
                    // if token invalid (400, 410):
                    let is_token_valid = true;

                    if !is_token_valid
                        && let Err(e) =
                            sqlx::query("UPDATE user_sessions SET push_token = NULL WHERE id = $1")
                                .bind(session_id)
                                .execute(&pool)
                                .await
                    {
                        error!("Failed to remove invalid push token: {}", e);
                    }
                }
            }
            Ok(None) => {} // Timeout, just loop
            Err(e) => {
                error!("Redis BRPOP error: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
            }
        }
    }
}
