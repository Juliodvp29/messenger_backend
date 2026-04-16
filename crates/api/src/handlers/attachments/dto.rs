use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct CreateUploadUrlRequest {
    pub file_type: String,
    pub file_size: i64,
    #[serde(default)]
    pub chat_id: Option<Uuid>,
    #[serde(default)]
    pub file_name: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct CreateUploadUrlResponse {
    pub upload_url: String,
    pub file_url: String,
    pub attachment_id: Uuid,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ConfirmAttachmentRequest {
    pub attachment_id: Uuid,
    pub message_id: Uuid,
    pub encryption_key_enc: Option<String>,
    pub encryption_iv: Option<String>,
}
