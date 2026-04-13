-- 009_chats_and_settings.sql
-- ============================================================
-- CHATS
CREATE TABLE chats (
    id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    type          chat_type   NOT NULL,
    name          TEXT,
    description   TEXT,
    avatar_url    TEXT,
    created_by    UUID        REFERENCES users(id) ON DELETE SET NULL,
    invite_link   TEXT        UNIQUE,
    is_encrypted  BOOLEAN     NOT NULL DEFAULT TRUE,
    max_members   INTEGER,
    CONSTRAINT chk_group_has_name CHECK (type NOT IN ('group', 'channel') OR (name IS NOT NULL AND name <> '')),
    CONSTRAINT chk_private_no_invite CHECK (type != 'private' OR invite_link IS NULL),
    CONSTRAINT chk_max_members_scope CHECK (type IN ('group', 'channel') OR max_members IS NULL),
    created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at    TIMESTAMPTZ
);

CREATE TRIGGER chats_updated_at
    BEFORE UPDATE ON chats
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

CREATE INDEX idx_chats_invite_link ON chats (invite_link) WHERE invite_link IS NOT NULL;
CREATE INDEX idx_chats_created_by  ON chats (created_by);

-- CHAT_SETTINGS
CREATE TABLE chat_settings (
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    chat_id      UUID        NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    is_muted     BOOLEAN     NOT NULL DEFAULT FALSE,
    muted_until  TIMESTAMPTZ,
    is_pinned    BOOLEAN     NOT NULL DEFAULT FALSE,
    pin_order    INTEGER     NOT NULL DEFAULT 0,
    is_archived  BOOLEAN     NOT NULL DEFAULT FALSE,
    theme        TEXT,
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    PRIMARY KEY (user_id, chat_id)
);

CREATE TRIGGER chat_settings_updated_at
    BEFORE UPDATE ON chat_settings
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

CREATE INDEX idx_chat_settings_pinned ON chat_settings (user_id, pin_order) WHERE is_pinned = TRUE;
