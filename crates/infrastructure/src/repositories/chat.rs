use chrono::{DateTime, Utc};
use domain::chat::entity::{Chat, ChatPreview, ChatType};
use domain::chat::repository::{ChatCursor, ChatRepository};
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
}
