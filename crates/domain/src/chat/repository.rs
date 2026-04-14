use super::entity::{Chat, ChatMessage, ChatPreview, PendingAttachment};
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

#[derive(Debug, Clone)]
pub struct NewPendingAttachment {
    pub attachment_id: Uuid,
    pub uploader_id: Uuid,
    pub chat_id: Uuid,
    pub object_key: String,
    pub file_url: String,
    pub file_type: String,
    pub file_size: i64,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ConfirmAttachmentInput {
    pub attachment_id: Uuid,
    pub message_id: Uuid,
    pub uploader_id: Uuid,
    pub encryption_key_enc: Option<String>,
    pub encryption_iv: Option<String>,
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

    async fn create_pending_attachment(
        &self,
        input: NewPendingAttachment,
    ) -> DomainResult<PendingAttachment>;

    async fn get_pending_attachment_for_user(
        &self,
        attachment_id: Uuid,
        uploader_id: Uuid,
    ) -> DomainResult<Option<PendingAttachment>>;

    async fn confirm_attachment(&self, input: ConfirmAttachmentInput) -> DomainResult<()>;
}
