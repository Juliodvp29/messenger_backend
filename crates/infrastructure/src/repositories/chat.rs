use chrono::{DateTime, Utc};
use domain::chat::entity::PendingAttachment;
use domain::chat::entity::{Chat, ChatMessage, ChatPreview, ChatType};
use domain::chat::repository::{
    ChatCursor, ChatRepository, ConfirmAttachmentInput, MessageCursor, MessageDirection,
    NewMessage, NewPendingAttachment, MessageReaction,
};
use shared::error::{DomainError, DomainResult};
use sqlx::PgPool;
use uuid::Uuid;

pub struct PostgresChatRepository {
    pool: PgPool,
}

impl PostgresChatRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

fn map_message_row(
    row: (
        Uuid,
        Uuid,
        Option<Uuid>,
        Option<Uuid>,
        Option<String>,
        Option<String>,
        String,
        Option<serde_json::Value>,
        bool,
        DateTime<Utc>,
        Option<DateTime<Utc>>,
        Option<DateTime<Utc>>,
    ),
) -> ChatMessage {
    ChatMessage {
        id: row.0,
        chat_id: row.1,
        sender_id: row.2,
        reply_to_id: row.3,
        content_encrypted: row.4,
        content_iv: row.5,
        message_type: row.6,
        metadata: row.7,
        is_forwarded: row.8,
        created_at: row.9,
        edited_at: row.10,
        deleted_at: row.11,
    }
}

impl ChatRepository for PostgresChatRepository {
    async fn create_private_chat(
        &self,
        creator_id: Uuid,
        participant_id: Uuid,
    ) -> DomainResult<Chat> {
        let existing = sqlx::query_as::<_, (Uuid,)>(
            r#"
            SELECT c.id
            FROM chats c
            JOIN chat_participants cp1 ON cp1.chat_id = c.id
            JOIN chat_participants cp2 ON cp2.chat_id = c.id
            WHERE c.type = 'private'
              AND c.deleted_at IS NULL
              AND cp1.user_id = $1
              AND cp2.user_id = $2
              AND cp1.left_at IS NULL
              AND cp2.left_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(creator_id)
        .bind(participant_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let chat_id = if let Some((chat_id,)) = existing {
            chat_id
        } else {
            let mut tx = self
                .pool
                .begin()
                .await
                .map_err(|e| DomainError::Internal(e.to_string()))?;

            let (new_chat_id,) = sqlx::query_as::<_, (Uuid,)>(
                r#"
                INSERT INTO chats (type, created_by)
                VALUES ('private', $1)
                RETURNING id
                "#,
            )
            .bind(creator_id)
            .fetch_one(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

            sqlx::query(
                r#"
                INSERT INTO chat_participants (chat_id, user_id, role, added_by)
                VALUES
                    ($1, $2, 'member', $2),
                    ($1, $3, 'member', $2)
                "#,
            )
            .bind(new_chat_id)
            .bind(creator_id)
            .bind(participant_id)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

            tx.commit()
                .await
                .map_err(|e| DomainError::Internal(e.to_string()))?;

            new_chat_id
        };

        self.get_chat_for_user(creator_id, chat_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("chat not found".to_string()))
    }

    async fn create_group_chat(
        &self,
        creator_id: Uuid,
        name: &str,
        participant_ids: &[Uuid],
    ) -> DomainResult<Chat> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let (chat_id,) = sqlx::query_as::<_, (Uuid,)>(
            r#"
            INSERT INTO chats (type, name, created_by)
            VALUES ('group', $1, $2)
            RETURNING id
            "#,
        )
        .bind(name)
        .bind(creator_id)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        sqlx::query(
            r#"
            INSERT INTO chat_participants (chat_id, user_id, role, added_by)
            VALUES ($1, $2, 'owner', $2)
            "#,
        )
        .bind(chat_id)
        .bind(creator_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if !participant_ids.is_empty() {
            sqlx::query(
                r#"
                INSERT INTO chat_participants (chat_id, user_id, role, added_by)
                SELECT $1, p.user_id, 'member', $2
                FROM UNNEST($3::uuid[]) AS p(user_id)
                WHERE p.user_id <> $2
                ON CONFLICT (chat_id, user_id) DO NOTHING
                "#,
            )
            .bind(chat_id)
            .bind(creator_id)
            .bind(participant_ids)
            .execute(&mut *tx)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        self.get_chat_for_user(creator_id, chat_id)
            .await?
            .ok_or_else(|| DomainError::NotFound("chat not found".to_string()))
    }

    async fn get_chat_for_user(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<Option<Chat>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<Uuid>,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT c.id, c.type::text, c.name, c.avatar_url, c.created_by, c.created_at
            FROM chats c
            JOIN chat_participants cp ON cp.chat_id = c.id
            WHERE c.id = $1
              AND cp.user_id = $2
              AND cp.left_at IS NULL
              AND c.deleted_at IS NULL
            "#,
        )
        .bind(chat_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        row.map(
            |(id, chat_type, name, avatar_url, created_by, created_at)| {
                let parsed_type = ChatType::from_db_str(&chat_type).ok_or_else(|| {
                    DomainError::Internal(format!("invalid chat type: {chat_type}"))
                })?;
                Ok(Chat {
                    id,
                    chat_type: parsed_type,
                    name,
                    avatar_url,
                    created_by,
                    created_at,
                })
            },
        )
        .transpose()
    }

    async fn list_chats_for_user(
        &self,
        user_id: Uuid,
        cursor: Option<ChatCursor>,
        limit: i64,
    ) -> DomainResult<Vec<ChatPreview>> {
        let page_size = limit.clamp(1, 50);
        let has_cursor = cursor.is_some();
        let cursor_pinned = cursor.as_ref().map(|c| c.is_pinned).unwrap_or(false);
        let cursor_last_message_at = cursor.as_ref().and_then(|c| c.last_message_at);
        let cursor_chat_id = cursor.map(|c| c.chat_id);

        let rows = sqlx::query_as::<
            _,
            (
                Uuid,
                String,
                Option<String>,
                Option<String>,
                Option<Uuid>,
                Option<String>,
                Option<Uuid>,
                Option<DateTime<Utc>>,
                bool,
                i32,
                bool,
                bool,
                i64,
            ),
        >(
            r#"
            SELECT
                v.chat_id,
                v.type::text,
                v.name,
                v.avatar_url,
                v.last_message_id,
                v.last_message_encrypted,
                v.last_sender_id,
                v.last_message_at,
                COALESCE(v.is_pinned, false) AS is_pinned,
                COALESCE(v.pin_order, 0) AS pin_order,
                COALESCE(v.is_muted, false) AS is_muted,
                COALESCE(v.is_archived, false) AS is_archived,
                COALESCE(ms.unread_count, 0) AS unread_count
            FROM v_chat_previews v
            LEFT JOIN LATERAL (
                SELECT COUNT(*)::BIGINT AS unread_count
                FROM message_status ms
                JOIN messages m ON m.id = ms.message_id
                WHERE ms.user_id = $1
                  AND m.chat_id = v.chat_id
                  AND ms.status <> 'read'
                  AND m.deleted_at IS NULL
            ) ms ON TRUE
            WHERE v.user_id = $1
              AND (
                    $2::bool = false
                    OR (
                        (COALESCE(v.is_pinned, false)::int < $3::int)
                        OR (
                            COALESCE(v.is_pinned, false)::int = $3::int
                            AND COALESCE(v.last_message_at, 'epoch'::timestamptz) < COALESCE($4::timestamptz, 'epoch'::timestamptz)
                        )
                        OR (
                            COALESCE(v.is_pinned, false)::int = $3::int
                            AND COALESCE(v.last_message_at, 'epoch'::timestamptz) = COALESCE($4::timestamptz, 'epoch'::timestamptz)
                            AND v.chat_id < $5::uuid
                        )
                    )
              )
            ORDER BY
                COALESCE(v.is_pinned, false) DESC,
                COALESCE(v.last_message_at, 'epoch'::timestamptz) DESC,
                v.chat_id DESC
            LIMIT $6
            "#,
        )
        .bind(user_id)
        .bind(has_cursor)
        .bind(if cursor_pinned { 1 } else { 0 })
        .bind(cursor_last_message_at)
        .bind(cursor_chat_id.unwrap_or_else(Uuid::nil))
        .bind(page_size)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        rows.into_iter()
            .map(
                |(
                    chat_id,
                    chat_type,
                    name,
                    avatar_url,
                    last_message_id,
                    last_message_encrypted,
                    last_sender_id,
                    last_message_at,
                    is_pinned,
                    pin_order,
                    is_muted,
                    is_archived,
                    unread_count,
                )| {
                    let parsed_type = ChatType::from_db_str(&chat_type).ok_or_else(|| {
                        DomainError::Internal(format!("invalid chat type in preview: {chat_type}"))
                    })?;
                    Ok(ChatPreview {
                        chat_id,
                        chat_type: parsed_type,
                        name,
                        avatar_url,
                        last_message_id,
                        last_message_encrypted,
                        last_sender_id,
                        last_message_at,
                        is_pinned,
                        pin_order,
                        is_muted,
                        is_archived,
                        unread_count,
                    })
                },
            )
            .collect()
    }

    async fn send_message(
        &self,
        sender_id: Uuid,
        chat_id: Uuid,
        message: NewMessage,
    ) -> DomainResult<ChatMessage> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let chat_type = ensure_active_membership(&mut tx, sender_id, chat_id).await?;
        if chat_type == "private" {
            ensure_not_blocked_in_private_chat(&mut tx, sender_id, chat_id).await?;
        }

        if message.message_type == "text"
            && (message.content_encrypted.is_none() || message.content_iv.is_none())
        {
            return Err(DomainError::Validation(
                "text messages require content_encrypted and content_iv".to_string(),
            ));
        }

        let inserted = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                Option<String>,
                Option<String>,
                String,
                Option<serde_json::Value>,
                bool,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
                Option<DateTime<Utc>>,
            ),
        >(
            r#"
            INSERT INTO messages (
                chat_id, sender_id, reply_to_id, content_encrypted, content_iv,
                message_type, metadata, is_forwarded
            )
            VALUES ($1, $2, $3, $4, $5, $6::message_type, $7, $8)
            RETURNING
                id, chat_id, sender_id, reply_to_id, content_encrypted, content_iv,
                message_type::text, metadata, is_forwarded, created_at, edited_at, deleted_at
            "#,
        )
        .bind(chat_id)
        .bind(sender_id)
        .bind(message.reply_to_id)
        .bind(message.content_encrypted)
        .bind(message.content_iv)
        .bind(message.message_type)
        .bind(message.metadata)
        .bind(message.is_forwarded)
        .fetch_one(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let message_id = inserted.0;
        sqlx::query(
            r#"
            INSERT INTO message_status (message_id, user_id, status)
            SELECT $1, cp.user_id, 'sent'
            FROM chat_participants cp
            WHERE cp.chat_id = $2
              AND cp.left_at IS NULL
            ON CONFLICT (message_id, user_id) DO NOTHING
            "#,
        )
        .bind(message_id)
        .bind(chat_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        tx.commit()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(map_message_row(inserted))
    }

    async fn list_messages(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        cursor: Option<MessageCursor>,
        direction: MessageDirection,
        limit: i64,
    ) -> DomainResult<Vec<ChatMessage>> {
        ensure_active_membership_pool(&self.pool, user_id, chat_id).await?;

        let page_size = limit.clamp(1, 50);
        let cursor_ts = cursor.as_ref().map(|c| c.created_at);
        let cursor_id = cursor.as_ref().map(|c| c.message_id);

        let direction_str = match direction {
            MessageDirection::Before => "before",
            MessageDirection::After => "after",
        };

        let mut rows = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                Option<String>,
                Option<String>,
                String,
                Option<serde_json::Value>,
                bool,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
                Option<DateTime<Utc>>,
            ),
        >(
            r#"
            SELECT
                id, chat_id, sender_id, reply_to_id, content_encrypted, content_iv,
                message_type::text, metadata, is_forwarded, created_at, edited_at, deleted_at
            FROM messages
            WHERE chat_id = $1
              AND deleted_at IS NULL
              AND (
                    $2::timestamptz IS NULL
                    OR (
                        $4::text = 'before'
                        AND (created_at, id) < ($2::timestamptz, $3::uuid)
                    )
                    OR (
                        $4::text = 'after'
                        AND (created_at, id) > ($2::timestamptz, $3::uuid)
                    )
              )
            ORDER BY
                CASE WHEN $4::text = 'before' THEN created_at END DESC,
                CASE WHEN $4::text = 'before' THEN id END DESC,
                CASE WHEN $4::text = 'after' THEN created_at END ASC,
                CASE WHEN $4::text = 'after' THEN id END ASC
            LIMIT $5
            "#,
        )
        .bind(chat_id)
        .bind(cursor_ts)
        .bind(cursor_id.unwrap_or_else(Uuid::nil))
        .bind(direction_str)
        .bind(page_size)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if matches!(direction, MessageDirection::Before) {
            rows.reverse();
        }

        Ok(rows.into_iter().map(map_message_row).collect())
    }

    async fn create_pending_attachment(
        &self,
        input: NewPendingAttachment,
    ) -> DomainResult<PendingAttachment> {
        ensure_active_membership_pool(&self.pool, input.uploader_id, input.chat_id).await?;

        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                String,
                String,
                String,
                i64,
                Option<String>,
                bool,
                DateTime<Utc>,
            ),
        >(
            r#"
            INSERT INTO message_attachments (
                id, message_id, uploader_id, chat_id, object_key, file_url, file_type, file_size, file_name, confirmed
            )
            VALUES ($1, NULL, $2, $3, $4, $5, $6, $7, $8, FALSE)
            RETURNING id, uploader_id, chat_id, object_key, file_url, file_type, file_size, file_name, confirmed, created_at
            "#,
        )
        .bind(input.attachment_id)
        .bind(input.uploader_id)
        .bind(input.chat_id)
        .bind(input.object_key)
        .bind(input.file_url)
        .bind(input.file_type)
        .bind(input.file_size)
        .bind(input.file_name)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(PendingAttachment {
            id: row.0,
            uploader_id: row.1,
            chat_id: row.2,
            object_key: row.3,
            file_url: row.4,
            file_type: row.5,
            file_size: row.6,
            file_name: row.7,
            confirmed: row.8,
            created_at: row.9,
        })
    }

    async fn get_pending_attachment_for_user(
        &self,
        attachment_id: Uuid,
        uploader_id: Uuid,
    ) -> DomainResult<Option<PendingAttachment>> {
        let row = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Uuid,
                String,
                String,
                String,
                i64,
                Option<String>,
                bool,
                DateTime<Utc>,
            ),
        >(
            r#"
            SELECT
                id, uploader_id, chat_id, object_key, file_url, file_type, file_size, file_name, confirmed, created_at
            FROM message_attachments
            WHERE id = $1
              AND uploader_id = $2
              AND confirmed = FALSE
            "#,
        )
        .bind(attachment_id)
        .bind(uploader_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(row.map(|r| PendingAttachment {
            id: r.0,
            uploader_id: r.1,
            chat_id: r.2,
            object_key: r.3,
            file_url: r.4,
            file_type: r.5,
            file_size: r.6,
            file_name: r.7,
            confirmed: r.8,
            created_at: r.9,
        }))
    }

    async fn confirm_attachment(&self, input: ConfirmAttachmentInput) -> DomainResult<()> {
        let mut tx = self
            .pool
            .begin()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

        let chat_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT m.chat_id
            FROM messages m
            WHERE m.id = $1
              AND m.deleted_at IS NULL
            "#,
        )
        .bind(input.message_id)
        .fetch_optional(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound("message not found".to_string()))?;

        ensure_active_membership(&mut tx, input.uploader_id, chat_id).await?;

        let updated = sqlx::query(
            r#"
            UPDATE message_attachments
            SET message_id = $2,
                encryption_key_enc = $3,
                encryption_iv = $4,
                confirmed = TRUE,
                confirmed_at = NOW()
            WHERE id = $1
              AND uploader_id = $5
              AND chat_id = $6
              AND confirmed = FALSE
            "#,
        )
        .bind(input.attachment_id)
        .bind(input.message_id)
        .bind(input.encryption_key_enc)
        .bind(input.encryption_iv)
        .bind(input.uploader_id)
        .bind(chat_id)
        .execute(&mut *tx)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if updated.rows_affected() == 0 {
            return Err(DomainError::NotFound(
                "pending attachment not found for message".to_string(),
            ));
        }

        tx.commit()
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        Ok(())
    }

    async fn verify_participant(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<()> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM chat_participants cp
            JOIN chats c ON c.id = cp.chat_id
            WHERE cp.chat_id = $1
              AND cp.user_id = $2
              AND cp.left_at IS NULL
              AND c.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(chat_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if exists.is_none() {
            return Err(DomainError::NotFound(
                "chat not found or not a participant".to_string(),
            ));
        }
        Ok(())
    }

    async fn verify_message_in_chat(&self, message_id: Uuid, chat_id: Uuid) -> DomainResult<()> {
        let exists = sqlx::query_scalar::<_, i64>(
            r#"
            SELECT 1
            FROM messages m
            WHERE m.id = $1
              AND m.chat_id = $2
              AND m.deleted_at IS NULL
            LIMIT 1
            "#,
        )
        .bind(message_id)
        .bind(chat_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if exists.is_none() {
            return Err(DomainError::NotFound(
                "message not found or not in chat".to_string(),
            ));
        }
        Ok(())
    }

    async fn mark_messages_read(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        up_to: DateTime<Utc>,
    ) -> DomainResult<i32> {
        let updated_count = sqlx::query_scalar::<_, i32>(
            r#"
            SELECT mark_messages_read($1, $2, $3)
            "#,
        )
        .bind(user_id)
        .bind(chat_id)
        .bind(up_to)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(updated_count)
    }

    async fn add_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        reaction: String,
    ) -> DomainResult<MessageReaction> {
        let reaction_record = sqlx::query_as::<_, (Uuid, Uuid, Uuid, String, DateTime<Utc>)>(
            r#"
            INSERT INTO message_reactions (message_id, user_id, reaction)
            VALUES ($1, $2, $3)
            ON CONFLICT (message_id, user_id, reaction) DO NOTHING
            RETURNING id, message_id, user_id, reaction, created_at
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(&reaction)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(MessageReaction {
            id: reaction_record.0,
            message_id: reaction_record.1,
            user_id: reaction_record.2,
            reaction: reaction_record.3,
            created_at: reaction_record.4,
        })
    }

    async fn remove_reaction(
        &self,
        message_id: Uuid,
        user_id: Uuid,
        reaction: &str,
    ) -> DomainResult<bool> {
        let result = sqlx::query(
            r#"
            DELETE FROM message_reactions
            WHERE message_id = $1
              AND user_id = $2
              AND reaction = $3
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(reaction)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn update_chat(
        &self,
        user_id: Uuid,
        chat_id: Uuid,
        name: Option<String>,
        avatar_url: Option<String>,
    ) -> DomainResult<Chat> {
        let (chat_type, current_name, current_avatar, created_by) = sqlx::query_as::<_, (String, Option<String>, Option<String>, Option<Uuid>)>(
            r#"
            SELECT c.type::text, c.name, c.avatar_url, c.created_by
            FROM chats c
            JOIN chat_participants cp ON cp.chat_id = c.id
            WHERE c.id = $1
              AND cp.user_id = $2
              AND cp.left_at IS NULL
              AND c.deleted_at IS NULL
            "#,
        )
        .bind(chat_id)
        .bind(user_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let new_name = name.or(current_name.clone());
        let new_avatar = avatar_url.or(current_avatar.clone());

        let (id, updated_at) = sqlx::query_as::<_, (Uuid, DateTime<Utc>)>(
            r#"
            UPDATE chats
            SET name = $3, avatar_url = $4, updated_at = NOW()
            WHERE id = $1
            RETURNING id, updated_at
            "#,
        )
        .bind(chat_id)
        .bind(new_name.clone())
        .bind(new_avatar.clone())
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let parsed_type = ChatType::from_db_str(&chat_type)
            .ok_or_else(|| DomainError::Internal("invalid chat type".to_string()))?;

        Ok(Chat {
            id,
            chat_type: parsed_type,
            name: new_name,
            avatar_url: new_avatar,
            created_by,
            created_at: updated_at,
        })
    }

    async fn delete_chat(&self, user_id: Uuid, chat_id: Uuid) -> DomainResult<()> {
        let chat_exists = sqlx::query_scalar::<_, String>(
            r#"
            SELECT type::text
            FROM chats c
            JOIN chat_participants cp ON cp.chat_id = c.id
            WHERE c.id = $1
              AND cp.user_id = $2
              AND cp.left_at IS NULL
              AND c.deleted_at IS NULL
            "#,
        )
        .bind(chat_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        let chat_type = chat_exists
            .ok_or_else(|| DomainError::NotFound("chat not found or not a participant".to_string()))?;

        if chat_type == "private" {
            sqlx::query(
                r#"
                UPDATE chat_participants
                SET left_at = NOW()
                WHERE chat_id = $1 AND user_id = $2
                "#,
            )
            .bind(chat_id)
            .bind(user_id)
            .execute(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;
        } else {
            let is_owner = sqlx::query_scalar::<_, bool>(
                r#"
                SELECT TRUE
                FROM chat_participants
                WHERE chat_id = $1 AND user_id = $2 AND role = 'owner'
                "#,
            )
            .bind(chat_id)
            .bind(user_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| DomainError::Internal(e.to_string()))?;

            if is_owner.unwrap_or(false) {
                sqlx::query(
                    r#"
                    UPDATE chats SET deleted_at = NOW() WHERE id = $1
                    "#,
                )
                .bind(chat_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DomainError::Internal(e.to_string()))?;
            } else {
                sqlx::query(
                    r#"
                    UPDATE chat_participants SET left_at = NOW()
                    WHERE chat_id = $1 AND user_id = $2
                    "#,
                )
                .bind(chat_id)
                .bind(user_id)
                .execute(&self.pool)
                .await
                .map_err(|e| DomainError::Internal(e.to_string()))?;
            }
        }

        Ok(())
    }

    async fn edit_message(
        &self,
        user_id: Uuid,
        message_id: Uuid,
        content_encrypted: Option<String>,
        content_iv: Option<String>,
    ) -> DomainResult<ChatMessage> {
        let chat_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT m.chat_id
            FROM messages m
            WHERE m.id = $1
              AND m.sender_id = $2
              AND m.deleted_at IS NULL
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?
        .ok_or_else(|| DomainError::NotFound("message not found or not authorized".to_string()))?;

        ensure_active_membership_pool(&self.pool, user_id, chat_id).await?;

        if content_encrypted.is_none() && content_iv.is_none() {
            return Err(DomainError::Validation(
                "content_encrypted and content_iv are required".to_string(),
            ));
        }

        let updated = sqlx::query_as::<
            _,
            (
                Uuid,
                Uuid,
                Option<Uuid>,
                Option<Uuid>,
                Option<String>,
                Option<String>,
                String,
                Option<serde_json::Value>,
                bool,
                DateTime<Utc>,
                Option<DateTime<Utc>>,
                Option<DateTime<Utc>>,
            ),
        >(
            r#"
            UPDATE messages
            SET content_encrypted = COALESCE($3, content_encrypted),
                content_iv = COALESCE($4, content_iv),
                edited_at = NOW()
            WHERE id = $1
            RETURNING id, chat_id, sender_id, reply_to_id, content_encrypted, content_iv,
                      message_type::text, metadata, is_forwarded, created_at, edited_at, deleted_at
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .bind(content_encrypted)
        .bind(content_iv)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        Ok(map_message_row(updated))
    }

    async fn delete_message(&self, user_id: Uuid, message_id: Uuid) -> DomainResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE messages
            SET deleted_at = NOW(),
                content_encrypted = NULL,
                content_iv = NULL,
                message_type = 'deleted'::message_type
            WHERE id = $1
              AND sender_id = $2
              AND deleted_at IS NULL
            "#,
        )
        .bind(message_id)
        .bind(user_id)
        .execute(&self.pool)
        .await
        .map_err(|e| DomainError::Internal(e.to_string()))?;

        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound("message not found or not authorized".to_string()));
        }

Ok(())
    }
}

async fn ensure_active_membership(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    user_id: Uuid,
    chat_id: Uuid,
) -> DomainResult<String> {
    let chat_type = sqlx::query_scalar::<_, String>(
        r#"
        SELECT c.type::text
        FROM chats c
        JOIN chat_participants cp ON cp.chat_id = c.id
        WHERE c.id = $1
          AND c.deleted_at IS NULL
          AND cp.user_id = $2
          AND cp.left_at IS NULL
        "#,
    )
    .bind(chat_id)
    .bind(user_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    chat_type
        .ok_or_else(|| DomainError::NotFound("chat not found or not a participant".to_string()))
}

async fn ensure_active_membership_pool(
    pool: &PgPool,
    user_id: Uuid,
    chat_id: Uuid,
) -> DomainResult<()> {
    let exists = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT 1
        FROM chat_participants cp
        JOIN chats c ON c.id = cp.chat_id
        WHERE cp.chat_id = $1
          AND cp.user_id = $2
          AND cp.left_at IS NULL
          AND c.deleted_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(chat_id)
    .bind(user_id)
    .fetch_optional(pool)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    if exists.is_none() {
        return Err(DomainError::NotFound(
            "chat not found or not a participant".to_string(),
        ));
    }
    Ok(())
}

async fn ensure_not_blocked_in_private_chat(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    sender_id: Uuid,
    chat_id: Uuid,
) -> DomainResult<()> {
    let blocked = sqlx::query_scalar::<_, i64>(
        r#"
        SELECT 1
        FROM chat_participants cp
        JOIN user_blocks ub
          ON (ub.blocker_id = $1 AND ub.blocked_id = cp.user_id)
          OR (ub.blocked_id = $1 AND ub.blocker_id = cp.user_id)
        WHERE cp.chat_id = $2
          AND cp.user_id <> $1
          AND cp.left_at IS NULL
        LIMIT 1
        "#,
    )
    .bind(sender_id)
    .bind(chat_id)
    .fetch_optional(&mut **tx)
    .await
    .map_err(|e| DomainError::Internal(e.to_string()))?;

    if blocked.is_some() {
        return Err(DomainError::Unauthorized(
            "cannot send messages in blocked private chat".to_string(),
        ));
    }
    Ok(())
}