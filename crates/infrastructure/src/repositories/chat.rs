use chrono::{DateTime, Utc};
use domain::chat::entity::{Chat, ChatMessage, ChatPreview, ChatType};
use domain::chat::repository::{
    ChatCursor, ChatRepository, MessageCursor, MessageDirection, NewMessage,
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
