use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ChatType {
    Private,
    Group,
    Channel,
    SelfChat,
}

impl ChatType {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Private => "private",
            Self::Group => "group",
            Self::Channel => "channel",
            Self::SelfChat => "self",
        }
    }

    pub fn from_db_str(value: &str) -> Option<Self> {
        match value {
            "private" => Some(Self::Private),
            "group" => Some(Self::Group),
            "channel" => Some(Self::Channel),
            "self" => Some(Self::SelfChat),
            _ => None,
        }
    }
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum ParticipantRole {
    Member = 1,
    Moderator = 2,
    Admin = 3,
    Owner = 4,
}

impl ParticipantRole {
    pub fn as_db_str(self) -> &'static str {
        match self {
            Self::Member => "member",
            Self::Moderator => "moderator",
            Self::Admin => "admin",
            Self::Owner => "owner",
        }
    }

    pub fn from_db_str(s: &str) -> Option<Self> {
        match s {
            "member" => Some(Self::Member),
            "moderator" => Some(Self::Moderator),
            "admin" => Some(Self::Admin),
            "owner" => Some(Self::Owner),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ParticipantDetail {
    pub user_id: Uuid,
    pub chat_id: Uuid,
    pub role: ParticipantRole,
    pub encryption_key_enc: Option<String>,
    pub added_by: Option<Uuid>,
    pub joined_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct Chat {
    pub id: Uuid,
    pub chat_type: ChatType,
    pub name: Option<String>,
    pub description: Option<String>,
    pub avatar_url: Option<String>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ChatPreview {
    pub chat_id: Uuid,
    pub chat_type: ChatType,
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

#[derive(Debug, Clone)]
pub struct ChatMessage {
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

#[derive(Debug, Clone)]
pub struct PendingAttachment {
    pub id: Uuid,
    pub uploader_id: Uuid,
    pub chat_id: Uuid,
    pub object_key: String,
    pub file_url: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_name: Option<String>,
    pub confirmed: bool,
    pub created_at: DateTime<Utc>,
}
