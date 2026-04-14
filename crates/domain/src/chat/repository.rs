use super::entity::{Chat, ChatMessage, ChatPreview, PendingAttachment};
use super::notifications::{
    ChatSettings, NewNotification, Notification, NotificationCursor, UpdateChatSettings,
};
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

#[derive(Debug, Clone)]
pub struct MessageReaction {
    pub id: Uuid,
    pub message_id: Uuid,
    pub user_id: Uuid,
    pub reaction: String,
    pub created_at: DateTime<Utc>,
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

    async fn verify_participant(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<()>;

    async fn verify_message_in_chat(&self, message_id: Uuid, chat_id: Uuid) -> DomainResult<()>;

    async fn mark_messages_read(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        up_to: DateTime<Utc>,
    ) -> DomainResult<i32>;

    async fn add_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        reaction: String,
    ) -> DomainResult<MessageReaction>;

    async fn remove_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        reaction: &str,
    ) -> DomainResult<bool>;

    async fn update_chat(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        name: Option<String>,
        avatar_url: Option<String>,
    ) -> DomainResult<Chat>;

    async fn delete_chat(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<()>;

    async fn edit_message(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        content_encrypted: Option<String>,
        content_iv: Option<String>,
    ) -> DomainResult<ChatMessage>;

    async fn delete_message(&self, user_id: Uuid, message_id: Uuid) -> DomainResult<()>;

    async fn get_chat_participants(&self, chat_id: Uuid) -> DomainResult<Vec<Uuid>>;

    async fn list_notifications(
        &self,
        user_id: Uuid,
        cursor: Option<NotificationCursor>,
        limit: i64,
    ) -> DomainResult<Vec<Notification>>;

    async fn create_notification(
        &self,
        notification: NewNotification,
    ) -> DomainResult<Notification>;

    async fn mark_notification_read(
        &self,
        user_id: Uuid,
        notification_id: Uuid,
    ) -> DomainResult<()>;

    async fn mark_all_notifications_read(&self, user_id: Uuid) -> DomainResult<i32>;

    async fn delete_read_notifications(&self, user_id: Uuid) -> DomainResult<i32>;

    async fn get_chat_settings(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
    ) -> DomainResult<Option<ChatSettings>>;

    async fn update_chat_settings(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        settings: UpdateChatSettings,
    ) -> DomainResult<ChatSettings>;
}
