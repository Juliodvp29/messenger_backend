-- 003_user_profiles.sql
-- ============================================================
-- USER_PROFILES
CREATE TABLE user_profiles (
    user_id                   UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    display_name              TEXT        NOT NULL DEFAULT '',
    bio                       TEXT        DEFAULT '',
    privacy_last_seen         privacy_level NOT NULL DEFAULT 'contacts',
    privacy_avatar            privacy_level NOT NULL DEFAULT 'everyone',
    privacy_status_text       privacy_level NOT NULL DEFAULT 'contacts',
    privacy_groups            privacy_level NOT NULL DEFAULT 'everyone',
    read_receipts_enabled     BOOLEAN     NOT NULL DEFAULT TRUE,
    online_presence_enabled   BOOLEAN     NOT NULL DEFAULT TRUE,
    disappearing_messages_ttl INTEGER,
    updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER user_profiles_updated_at
    BEFORE UPDATE ON user_profiles
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
