-- 006_user_sessions.sql
-- ============================================================
-- USER_SESSIONS
CREATE TABLE user_sessions (
    id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    device_id       TEXT        NOT NULL,
    device_name     TEXT        NOT NULL DEFAULT '',
    device_type     device_type NOT NULL,
    push_token      TEXT,
    ip_address      INET,
    user_agent      TEXT,
    last_active_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at      TIMESTAMPTZ NOT NULL,
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, device_id)
);

CREATE INDEX idx_sessions_user       ON user_sessions (user_id);
CREATE INDEX idx_sessions_expires    ON user_sessions (expires_at);
CREATE INDEX idx_sessions_device     ON user_sessions (device_id);
