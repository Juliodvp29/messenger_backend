-- 011_stories_notifications_audit.sql
-- ============================================================
-- STORIES
CREATE TABLE stories (
    id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id       UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    content_url   TEXT          NOT NULL,
    content_type  TEXT          NOT NULL DEFAULT 'image',
    caption       TEXT,
    privacy       story_privacy NOT NULL DEFAULT 'contacts',
    created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    expires_at    TIMESTAMPTZ   NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
    deleted_at    TIMESTAMPTZ
);

CREATE TABLE story_privacy_exceptions (
    story_id     UUID        NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    is_excluded  BOOLEAN     NOT NULL DEFAULT FALSE,
    PRIMARY KEY (story_id, user_id)
);

CREATE TABLE story_views (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    story_id     UUID        NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
    viewer_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction     TEXT,
    viewed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (story_id, viewer_id)
);

-- NOTIFICATIONS
CREATE TABLE notifications (
    id           UUID               PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    type         notification_type  NOT NULL,
    data         JSONB              NOT NULL DEFAULT '{}',
    is_read      BOOLEAN            NOT NULL DEFAULT FALSE,
    read_at      TIMESTAMPTZ,
    created_at   TIMESTAMPTZ        NOT NULL DEFAULT NOW()
);

-- AUDIT_LOG
CREATE TABLE audit_log (
    id          UUID        DEFAULT gen_random_uuid(),
    user_id     UUID,
    action      audit_action NOT NULL,
    entity_type TEXT,
    entity_id   UUID,
    metadata    JSONB       NOT NULL DEFAULT '{}',
    ip_address  INET,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (id, created_at)
) PARTITION BY RANGE (created_at);

-- Partition Logic (Simplified for migration)
CREATE OR REPLACE FUNCTION create_audit_log_partitions(months_ahead INTEGER DEFAULT 3)
    RETURNS void AS $$
DECLARE
    start_date  DATE;
    end_date    DATE;
    part_name   TEXT;
    cur_month   DATE := DATE_TRUNC('month', NOW())::DATE;
BEGIN
    FOR i IN 0..months_ahead LOOP
        start_date := cur_month + (i * INTERVAL '1 month');
        end_date   := start_date + INTERVAL '1 month';
        part_name  := 'audit_log_' || TO_CHAR(start_date, 'YYYY_MM');
        IF NOT EXISTS (SELECT 1 FROM pg_class c JOIN pg_namespace n ON n.oid = c.relnamespace WHERE c.relname = part_name AND n.nspname = current_schema()) THEN
            EXECUTE format('CREATE TABLE %I PARTITION OF audit_log FOR VALUES FROM (%L) TO (%L)', part_name, start_date, end_date);
        END IF;
    END LOOP;
END;
$$ LANGUAGE plpgsql;

SELECT create_audit_log_partitions(6);

-- RLS
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;
CREATE POLICY messages_select ON messages FOR SELECT USING (chat_id IN (SELECT chat_id FROM chat_participants WHERE user_id = current_setting('app.current_user_id', true)::UUID AND left_at IS NULL));
CREATE POLICY messages_update ON messages FOR UPDATE USING (sender_id = current_setting('app.current_user_id', true)::UUID);

ALTER TABLE user_sessions ENABLE ROW LEVEL SECURITY;
CREATE POLICY sessions_select ON user_sessions FOR SELECT USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- VIEWS
CREATE VIEW v_chat_previews AS
SELECT cp.user_id, c.id AS chat_id, c.type, c.name, c.avatar_url, m.id AS last_message_id, m.message_type, m.created_at AS last_message_at
FROM chat_participants cp
JOIN chats c ON c.id = cp.chat_id AND c.deleted_at IS NULL
LEFT JOIN LATERAL (SELECT id, message_type, created_at FROM messages WHERE chat_id = c.id AND deleted_at IS NULL ORDER BY created_at DESC LIMIT 1) m ON TRUE
WHERE cp.left_at IS NULL;

-- UTILITIES
CREATE OR REPLACE FUNCTION get_user_public_keys(target_user_id UUID)
    RETURNS TABLE (identity_key TEXT, signed_prekey TEXT, signed_prekey_id INTEGER, signed_prekey_sig TEXT, one_time_prekey_id INTEGER, one_time_prekey TEXT) AS $$
DECLARE
    v_opk_id     BIGINT;
    v_opk_key_id INTEGER;
    v_opk_key    TEXT;
BEGIN
    SELECT id, key_id, public_key INTO v_opk_id, v_opk_key_id, v_opk_key FROM one_time_prekeys WHERE user_id = target_user_id AND is_consumed = FALSE ORDER BY id LIMIT 1 FOR UPDATE SKIP LOCKED;
    IF v_opk_id IS NOT NULL THEN
        UPDATE one_time_prekeys SET is_consumed = TRUE, consumed_at = NOW() WHERE id = v_opk_id;
        UPDATE user_keys SET prekey_count = GREATEST(prekey_count - 1, 0), updated_at = NOW() WHERE user_id = target_user_id;
    END IF;
    RETURN QUERY SELECT uk.identity_key, uk.signed_prekey, uk.signed_prekey_id, uk.signed_prekey_sig, v_opk_key_id, v_opk_key FROM user_keys uk WHERE uk.user_id = target_user_id;
END;
$$ LANGUAGE plpgsql;
