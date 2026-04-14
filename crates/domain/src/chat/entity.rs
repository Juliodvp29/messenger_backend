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

#[derive(Debug, Clone)]
pub struct Chat {
    pub id: Uuid,
    pub chat_type: ChatType,
    pub name: Option<String>,
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
