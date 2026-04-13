-- 002_users.sql
-- ============================================================
-- USERS
CREATE TABLE users (
    id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    username          TEXT        UNIQUE,
    phone             TEXT        UNIQUE NOT NULL,
    email             TEXT        UNIQUE,
    -- password_hash eliminado (Fase 03: OTP + PIN local)
    avatar_url        TEXT,
    status_text       TEXT        DEFAULT '',
    two_fa_enabled    BOOLEAN     NOT NULL DEFAULT FALSE,
    two_fa_secret     TEXT,
    is_active         BOOLEAN     NOT NULL DEFAULT TRUE,
    last_seen_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    deleted_at        TIMESTAMPTZ
);

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- Índices users
CREATE INDEX idx_users_phone       ON users (phone)       WHERE deleted_at IS NULL;
CREATE INDEX idx_users_username    ON users (username)    WHERE deleted_at IS NULL AND username IS NOT NULL;
CREATE INDEX idx_users_last_seen   ON users (last_seen_at DESC NULLS LAST);
