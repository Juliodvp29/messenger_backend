-- 012_message_attachments_pending_upload.sql
-- Enables two-step attachment flow:
-- 1) create pending attachment with presigned URL
-- 2) confirm after object exists in S3/MinIO

ALTER TABLE message_attachments
    ALTER COLUMN message_id DROP NOT NULL;

ALTER TABLE message_attachments
    ADD COLUMN IF NOT EXISTS uploader_id UUID REFERENCES users(id) ON DELETE SET NULL,
    ADD COLUMN IF NOT EXISTS chat_id UUID REFERENCES chats(id) ON DELETE CASCADE,
    ADD COLUMN IF NOT EXISTS object_key TEXT,
    ADD COLUMN IF NOT EXISTS confirmed BOOLEAN NOT NULL DEFAULT FALSE,
    ADD COLUMN IF NOT EXISTS confirmed_at TIMESTAMPTZ;

CREATE INDEX IF NOT EXISTS idx_attachments_uploader_pending
    ON message_attachments (uploader_id, created_at DESC)
    WHERE confirmed = FALSE;

CREATE INDEX IF NOT EXISTS idx_attachments_chat
    ON message_attachments (chat_id, created_at DESC);
