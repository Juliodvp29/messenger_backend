use super::entity::{Chat, ChatMessage, ChatPreview};
use chrono::{DateTime, Utc};
use shared::error::DomainResult;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct ChatCursor {
    pub is_pinned: bool,
    pub last_message_at: Option<DateTime<Utc>>,
    pub chat_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct MessageCursor {
    pub created_at: DateTime<Utc>,
    pub message_id: Uuid,
}

#[derive(Debug, Clone, Copy)]
pub enum MessageDirection {
    Before,
    After,
}

#[derive(Debug, Clone)]
pub struct NewMessage {
    pub content_encrypted: Option<String>,
    pub content_iv: Option<String>,
    pub message_type: String,
    pub reply_to_id: Option<Uuid>,
    pub is_forwarded: bool,
    pub metadata: Option<serde_json::Value>,
}

#[allow(async_fn_in_trait)]
pub trait ChatRepository: Send + Sync {
    async fn create_private_chat(
        &self,
        creator_id: Uuid,
        participant_id: Uuid,
    ) -> DomainResult<Chat>;

    async fn create_group_chat(
        &self,
        creator_id: Uuid,
        name: &str,
        participant_ids: &[Uuid],
    ) -> DomainResult<Chat>;

    async fn get_chat_for_user(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<Option<Chat>>;

    async fn list_chats_for_user(
        &self,
        user_id: Uuid,
        cursor: Option<ChatCursor>,
        limit: i64,
    ) -> DomainResult<Vec<ChatPreview>>;

    async fn send_message(
        &self,
        sender_id: Uuid,
        chat_id: Uuid,
        message: NewMessage,
    ) -> DomainResult<ChatMessage>;

    async fn list_messages(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        cursor: Option<MessageCursor>,
        direction: MessageDirection,
        limit: i64,
    ) -> DomainResult<Vec<ChatMessage>>;
}
