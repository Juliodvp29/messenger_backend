-- 010_participants_and_messages.sql
-- ============================================================
-- CHAT_PARTICIPANTS
CREATE TABLE chat_participants (
    id                  UUID              PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id             UUID              NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    user_id             UUID              NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role                participant_role  NOT NULL DEFAULT 'member',
    encryption_key_enc  TEXT,
    added_by            UUID              REFERENCES users(id) ON DELETE SET NULL,
    joined_at           TIMESTAMPTZ       NOT NULL DEFAULT NOW(),
    left_at             TIMESTAMPTZ,
    disappearing_ttl    INTEGER,
    UNIQUE (chat_id, user_id)
);

CREATE OR REPLACE FUNCTION enforce_private_chat_limit()
    RETURNS TRIGGER AS $$
DECLARE
    chat_kind         chat_type;
    participant_count INTEGER;
BEGIN
    SELECT type INTO chat_kind FROM chats WHERE id = NEW.chat_id;
    IF chat_kind = 'private' THEN
        SELECT COUNT(*) INTO participant_count
        FROM chat_participants
        WHERE chat_id = NEW.chat_id AND left_at IS NULL;
        IF participant_count >= 2 THEN
            RAISE EXCEPTION 'Un chat privado no puede tener más de 2 participantes activos' USING ERRCODE = 'check_violation';
        END IF;
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_enforce_private_chat_limit
    BEFORE INSERT ON chat_participants
    FOR EACH ROW EXECUTE FUNCTION enforce_private_chat_limit();

-- MESSAGES
CREATE TABLE messages (
    id                UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
    chat_id           UUID          NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    sender_id         UUID          REFERENCES users(id) ON DELETE SET NULL,
    reply_to_id       UUID          REFERENCES messages(id) ON DELETE SET NULL,
    content_encrypted TEXT,
    content_iv        TEXT,
    message_type      message_type  NOT NULL DEFAULT 'text',
    metadata          JSONB,
    is_forwarded      BOOLEAN       NOT NULL DEFAULT FALSE,
    created_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
    edited_at         TIMESTAMPTZ,
    deleted_at        TIMESTAMPTZ,
    expires_at        TIMESTAMPTZ,
    CONSTRAINT chk_text_message_has_content CHECK (message_type != 'text' OR deleted_at IS NOT NULL OR (content_encrypted IS NOT NULL AND content_iv IS NOT NULL)),
    CONSTRAINT chk_content_iv_consistency CHECK ((content_encrypted IS NULL) = (content_iv IS NULL))
);

CREATE INDEX idx_messages_chat_created    ON messages (chat_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_messages_chat_id         ON messages (chat_id) WHERE deleted_at IS NULL;
CREATE INDEX idx_messages_sender          ON messages (sender_id, created_at DESC) WHERE deleted_at IS NULL AND sender_id IS NOT NULL;

-- MESSAGE_ATTACHMENTS
CREATE TABLE message_attachments (
    id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id           UUID        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    file_url             TEXT        NOT NULL,
    file_type            TEXT        NOT NULL,
    file_size            BIGINT      NOT NULL,
    file_name            TEXT,
    thumbnail_url        TEXT,
    width                INTEGER,
    height               INTEGER,
    duration_seconds     INTEGER,
    encryption_key_enc   TEXT,
    encryption_iv        TEXT,
    created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- MESSAGE_REACTIONS
CREATE TABLE message_reactions (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    message_id   UUID        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    reaction     TEXT        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (message_id, user_id, reaction)
);

-- MESSAGE_STATUS
CREATE TABLE message_status (
    message_id   UUID           NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
    user_id      UUID           NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    status       message_status_enum  NOT NULL DEFAULT 'sent',
    updated_at   TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
    PRIMARY KEY (message_id, user_id)
);

CREATE INDEX idx_msg_status_user    ON message_status (user_id, updated_at DESC);
