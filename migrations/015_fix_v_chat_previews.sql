DROP VIEW IF EXISTS v_chat_previews;
CREATE VIEW v_chat_previews AS
SELECT
    cp.user_id,
    c.id         AS chat_id,
    c.type,
    c.name,
    c.avatar_url,
    m.id         AS last_message_id,
    m.message_type,
    m.content_encrypted AS last_message_encrypted,
    m.sender_id  AS last_sender_id,
    m.created_at AS last_message_at,
    cs.is_pinned,
    cs.pin_order,
    cs.is_muted,
    cs.is_archived
FROM chat_participants cp
JOIN chats c ON c.id = cp.chat_id AND c.deleted_at IS NULL
LEFT JOIN LATERAL (
    SELECT id, message_type, content_encrypted, sender_id, created_at
    FROM messages
    WHERE chat_id = c.id AND deleted_at IS NULL
    ORDER BY created_at DESC
    LIMIT 1
) m ON TRUE
LEFT JOIN chat_settings cs ON cs.chat_id = c.id AND cs.user_id = cp.user_id
WHERE cp.left_at IS NULL;