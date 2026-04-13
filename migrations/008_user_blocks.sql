-- 008_user_blocks.sql
-- ============================================================
-- USER_BLOCKS
CREATE TABLE user_blocks (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    blocker_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    blocked_id   UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (blocker_id, blocked_id),
    CHECK (blocker_id <> blocked_id)
);

CREATE INDEX idx_blocks_blocker ON user_blocks (blocker_id);
CREATE INDEX idx_blocks_blocked ON user_blocks (blocked_id);
