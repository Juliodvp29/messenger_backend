-- Migration 016: Group Participant Management indexes
-- Optimises queries introduced in Phase 9 (groups & channels).

-- Active participant lookup by role (used in role-based authorization checks)
CREATE INDEX IF NOT EXISTS idx_chat_participants_role
    ON chat_participants (chat_id, role)
    WHERE left_at IS NULL;

-- Fast join-by-slug lookup (one-column index on the invite link slug)
CREATE INDEX IF NOT EXISTS idx_chats_invite_link
    ON chats (invite_link)
    WHERE invite_link IS NOT NULL AND deleted_at IS NULL;

-- Full-group participant count (used when checking max_members)
CREATE INDEX IF NOT EXISTS idx_chat_participants_active
    ON chat_participants (chat_id)
    WHERE left_at IS NULL;
