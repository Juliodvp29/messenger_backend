-- 014_message_read_function.sql
-- ============================================================
-- Function to mark messages as read for a user in a chat

CREATE OR REPLACE FUNCTION mark_messages_read(
    p_user_id UUID,
    p_chat_id UUID,
    p_up_to TIMESTAMPTZ
) RETURNS INTEGER AS $$
DECLARE
    updated_count INTEGER;
BEGIN
    -- Update message_status for messages up to the specified timestamp
    UPDATE message_status
    SET status = 'read', updated_at = NOW()
    WHERE message_id IN (
        SELECT m.id
        FROM messages m
        JOIN chat_participants cp ON m.chat_id = cp.chat_id
        WHERE m.chat_id = p_chat_id
          AND m.sender_id != p_user_id  -- Don't mark own messages as read
          AND cp.user_id = p_user_id     -- User must be participant
          AND cp.left_at IS NULL         -- User must be active participant
          AND m.created_at <= p_up_to    -- Only messages up to timestamp
          AND m.deleted_at IS NULL       -- Only non-deleted messages
    )
    AND user_id = p_user_id
    AND (status != 'read' OR updated_at > p_up_to);  -- Only update if not already read
    
    GET DIAGNOSTICS updated_count = ROW_COUNT;
    
    RETURN updated_count;
END;
$$ LANGUAGE plpgsql;

-- Index for better performance on message_status queries
CREATE INDEX idx_msg_status_message_user_status 
    ON message_status (message_id, user_id, status) 
    WHERE status = 'sent';
