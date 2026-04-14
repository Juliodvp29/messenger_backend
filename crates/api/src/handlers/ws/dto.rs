use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsParams {
    pub token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsClientMessage {
    #[serde(rename = "typing_start")]
    TypingStart { chat_id: Uuid },
    #[serde(rename = "typing_stop")]
    TypingStop { chat_id: Uuid },
    #[serde(rename = "sync_request")]
    SyncRequest {
        since: Option<chrono::DateTime<chrono::Utc>>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload")]
pub enum WsServerMessage {
    #[serde(rename = "new_message")]
    NewMessage(NewMessagePayload),
    #[serde(rename = "message_edited")]
    MessageEdited(MessageEditedPayload),
    #[serde(rename = "message_deleted")]
    MessageDeleted(MessageDeletedPayload),
    #[serde(rename = "reaction_added")]
    ReactionAdded(ReactionPayload),
    #[serde(rename = "reaction_removed")]
    ReactionRemoved(ReactionPayload),
    #[serde(rename = "messages_read")]
    MessagesRead(MessagesReadPayload),
    #[serde(rename = "user_online")]
    UserOnline(UserPresencePayload),
    #[serde(rename = "user_offline")]
    UserOffline(UserPresencePayload),
    #[serde(rename = "typing_start")]
    TypingStart(TypingPayload),
    #[serde(rename = "typing_stop")]
    TypingStop(TypingPayload),
    #[serde(rename = "key_changed")]
    KeyChanged(KeyChangedPayload),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMessagePayload {
    pub chat_id: Uuid,
    pub message: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageEditedPayload {
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub content_encrypted: String,
    pub content_iv: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessageDeletedPayload {
    pub chat_id: Uuid,
    pub message_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReactionPayload {
    pub chat_id: Uuid,
    pub message_id: Uuid,
    pub reaction: String,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MessagesReadPayload {
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub up_to: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPresencePayload {
    pub user_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypingPayload {
    pub chat_id: Uuid,
    pub user_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyChangedPayload {
    pub user_id: Uuid,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}
