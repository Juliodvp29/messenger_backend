use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum CreateChatRequest {
    Private {
        participant_id: Uuid,
    },
    Group {
        name: String,
        #[serde(default)]
        participant_ids: Vec<Uuid>,
    },
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: Uuid,
    pub chat_type: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct ListChatsQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct ChatPreviewResponse {
    pub chat_id: Uuid,
    pub chat_type: String,
    pub name: Option<String>,
    pub avatar_url: Option<String>,
    pub last_message_id: Option<Uuid>,
    pub last_message_encrypted: Option<String>,
    pub last_sender_id: Option<Uuid>,
    pub last_message_at: Option<DateTime<Utc>>,
    pub is_pinned: bool,
    pub pin_order: i32,
    pub is_muted: bool,
    pub is_archived: bool,
    pub unread_count: i64,
}

#[derive(Debug, Serialize)]
pub struct ListChatsResponse {
    pub items: Vec<ChatPreviewResponse>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatCursorDto {
    pub is_pinned: bool,
    pub last_message_at: Option<DateTime<Utc>>,
    pub chat_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct SendMessageRequest {
    pub content_encrypted: Option<String>,
    pub content_iv: Option<String>,
    pub message_type: String,
    pub reply_to_id: Option<Uuid>,
    pub is_forwarded: bool,
    pub metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Option<Uuid>,
    pub reply_to_id: Option<Uuid>,
    pub content_encrypted: Option<String>,
    pub content_iv: Option<String>,
    pub message_type: String,
    pub metadata: Option<serde_json::Value>,
    pub is_forwarded: bool,
    pub created_at: DateTime<Utc>,
    pub edited_at: Option<DateTime<Utc>>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct ListMessagesQuery {
    pub cursor: Option<String>,
    pub limit: Option<i64>,
    pub direction: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct ListMessagesResponse {
    pub items: Vec<MessageResponse>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MessageCursorDto {
    pub created_at: DateTime<Utc>,
    pub message_id: Uuid,
}
