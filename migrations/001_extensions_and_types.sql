-- 001_extensions_and_types.sql
-- ============================================================
-- Extensiones necesarias
CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- gen_random_uuid(), funciones hash
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- búsqueda de texto por trigrama
CREATE EXTENSION IF NOT EXISTS "btree_gin";  -- índices GIN para columnas combinadas

-- ============================================================
-- TIPOS ENUMERADOS
CREATE TYPE chat_type         AS ENUM ('private', 'group', 'channel', 'self');
CREATE TYPE participant_role  AS ENUM ('owner', 'admin', 'moderator', 'member');
CREATE TYPE message_type      AS ENUM ('text', 'image', 'video', 'audio', 'file', 'location', 'contact', 'sticker', 'deleted', 'system');
CREATE TYPE message_status_enum     AS ENUM ('sending', 'sent', 'delivered', 'read', 'failed');
CREATE TYPE story_privacy     AS ENUM ('everyone', 'contacts', 'contacts_except', 'only_me', 'selected');
CREATE TYPE device_type       AS ENUM ('android', 'ios', 'web', 'desktop');
CREATE TYPE notification_type AS ENUM ('message', 'reaction', 'group_invite', 'story_view', 'call', 'system');
CREATE TYPE privacy_level     AS ENUM ('everyone', 'contacts', 'nobody');
CREATE TYPE audit_action      AS ENUM (
    'login', 'logout', 'password_change', 'two_fa_enable', 'two_fa_disable',
    'account_delete', 'device_added', 'device_removed',
    'message_send', 'message_delete', 'message_edit',
    'group_create', 'group_update', 'group_delete',
    'participant_add', 'participant_remove', 'participant_role_change',
    'block_user', 'unblock_user', 'key_rotation'
);

-- ============================================================
-- FUNCIÓN AUXILIAR: updated_at automático
CREATE OR REPLACE FUNCTION trigger_set_updated_at()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;
