-- 004_user_keys.sql
-- ============================================================
-- USER_KEYS (Signal Protocol - E2E Encryption)
CREATE TABLE user_keys (
    user_id              UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
    identity_key         TEXT        NOT NULL,
    signed_prekey        TEXT        NOT NULL,
    signed_prekey_id     INTEGER     NOT NULL DEFAULT 1,
    signed_prekey_sig    TEXT        NOT NULL,
    prekey_count         INTEGER     NOT NULL DEFAULT 0,
    updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER user_keys_updated_at
    BEFORE UPDATE ON user_keys
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();
