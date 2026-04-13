-- 005_one_time_prekeys.sql
-- ============================================================
-- ONE_TIME_PREKEYS
CREATE TABLE one_time_prekeys (
    id           BIGSERIAL   PRIMARY KEY,
    user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    key_id       INTEGER     NOT NULL,
    public_key   TEXT        NOT NULL,
    is_consumed  BOOLEAN     NOT NULL DEFAULT FALSE,
    consumed_at  TIMESTAMPTZ,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (user_id, key_id)
);

CREATE INDEX idx_opk_available ON one_time_prekeys (user_id, id) WHERE is_consumed = FALSE;
CREATE INDEX idx_opk_consumed   ON one_time_prekeys (consumed_at) WHERE is_consumed = TRUE;
