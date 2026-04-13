-- ============================================================
--  SCHEMA: App de mensajería tipo WhatsApp/Telegram
--  Diseñado para: escalabilidad, E2E encryption, privacidad
-- ============================================================

DROP TYPE IF EXISTS chat_type CASCADE;
DROP TYPE IF EXISTS participant_role CASCADE;
DROP TYPE IF EXISTS message_type CASCADE;
DROP TYPE IF EXISTS message_status_enum  CASCADE;
DROP TYPE IF EXISTS story_privacy CASCADE;
DROP TYPE IF EXISTS device_type CASCADE;
DROP TYPE IF EXISTS notification_type CASCADE;
DROP TYPE IF EXISTS privacy_level CASCADE;
DROP TYPE IF EXISTS audit_action CASCADE;

-- Extensiones necesarias
CREATE EXTENSION IF NOT EXISTS "pgcrypto";   -- gen_random_uuid(), funciones hash
CREATE EXTENSION IF NOT EXISTS "pg_trgm";    -- búsqueda de texto por trigrama
CREATE EXTENSION IF NOT EXISTS "btree_gin";  -- índices GIN para columnas combinadas

-- ============================================================
--  TIPOS ENUMERADOS
--  Usar ENUM en lugar de TEXT libre asegura integridad desde
--  la capa de base de datos sin necesidad de CHECK constraints.
-- ============================================================

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
--  FUNCIÓN AUXILIAR: updated_at automático
-- ============================================================

CREATE OR REPLACE FUNCTION trigger_set_updated_at()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- ============================================================
--  USERS
--  Núcleo de toda la app. Soft delete con deleted_at.
--  El phone es el identificador principal (como WhatsApp).
--  El email es opcional (para recuperación de cuenta, 2FA).
-- ============================================================

CREATE TABLE users (
                       id                UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                       username          TEXT        UNIQUE,                         -- @handle único, opcional
                       phone             TEXT        UNIQUE NOT NULL,                -- número E.164: +573001234567
                       email             TEXT        UNIQUE,
                       -- password_hash eliminado (Fase 03: OTP + PIN local)
                       avatar_url        TEXT,
                       status_text       TEXT        DEFAULT '',                     -- "en reunión", "disponible"
                       two_fa_enabled    BOOLEAN     NOT NULL DEFAULT FALSE,
                       two_fa_secret     TEXT,                                       -- TOTP secret (cifrado en app)
                       is_active         BOOLEAN     NOT NULL DEFAULT TRUE,          -- cuenta suspendida/baneada
                       last_seen_at      TIMESTAMPTZ,                                -- controlado por privacy_profile
                       created_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                       updated_at        TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                       deleted_at        TIMESTAMPTZ                                 -- soft delete
);

CREATE TRIGGER users_updated_at
    BEFORE UPDATE ON users
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- Índices users
CREATE INDEX idx_users_phone       ON users (phone)       WHERE deleted_at IS NULL;
CREATE INDEX idx_users_username    ON users (username)    WHERE deleted_at IS NULL AND username IS NOT NULL;
CREATE INDEX idx_users_last_seen   ON users (last_seen_at DESC NULLS LAST);

-- ============================================================
--  USER_PROFILES
--  Separado de users para:
--  1. Separar autenticación de preferencias de privacidad.
--  2. Permite cachear el perfil público sin exponer datos sensibles.
-- ============================================================

CREATE TABLE user_profiles (
                               user_id                   UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                               display_name              TEXT        NOT NULL DEFAULT '',
                               bio                       TEXT        DEFAULT '',
                               privacy_last_seen         privacy_level NOT NULL DEFAULT 'contacts',
                               privacy_avatar            privacy_level NOT NULL DEFAULT 'everyone',
                               privacy_status_text       privacy_level NOT NULL DEFAULT 'contacts',
                               privacy_groups            privacy_level NOT NULL DEFAULT 'everyone', -- quién puede añadirte a grupos
                               read_receipts_enabled     BOOLEAN     NOT NULL DEFAULT TRUE,          -- tick doble azul
                               online_presence_enabled   BOOLEAN     NOT NULL DEFAULT TRUE,          -- mostrar "en línea"
                               disappearing_messages_ttl INTEGER,                                    -- segundos; NULL = off
                               updated_at                TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER user_profiles_updated_at
    BEFORE UPDATE ON user_profiles
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- ============================================================
--  USER_KEYS (Signal Protocol - E2E Encryption)
--  Implementa el protocolo X3DH (Extended Triple Diffie-Hellman).
--
--  identity_key:      clave de identidad de largo plazo (IK)
--  signed_prekey:     clave firmada de mediano plazo (SPK), se rota cada ~7 días
--  signed_prekey_sig: firma de SPK con IK, el cliente la verifica
--
--  ❌ DECISIÓN: Las one-time prekeys (OPKs) ya NO van en JSONB aquí.
--  Problema con JSONB:
--    - Rewrite completo de la fila en cada consumo de OPK
--    - FOR UPDATE bloquea toda la fila, incluyendo identity_key y SPK
--    - No se puede indexar individualmente cada prekey
--    - Con usuarios activos, genera write amplification severo
--  ✅ SOLUCIÓN: Tabla separada one_time_prekeys (ver más abajo).
--  prekey_count: contador denormalizado para saber cuántas OPKs
--  quedan sin tener que hacer COUNT(*) en cada request.
-- ============================================================

CREATE TABLE user_keys (
                           user_id              UUID        PRIMARY KEY REFERENCES users(id) ON DELETE CASCADE,
                           identity_key         TEXT        NOT NULL,                    -- clave pública IK (base64)
                           signed_prekey        TEXT        NOT NULL,                    -- clave pública SPK (base64)
                           signed_prekey_id     INTEGER     NOT NULL DEFAULT 1,          -- ID para rotar SPKs
                           signed_prekey_sig    TEXT        NOT NULL,                    -- firma de SPK con IK (base64)
                           prekey_count         INTEGER     NOT NULL DEFAULT 0,          -- contador denormalizado de OPKs
                           updated_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TRIGGER user_keys_updated_at
    BEFORE UPDATE ON user_keys
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

-- ============================================================
--  ONE_TIME_PREKEYS
--  Tabla dedicada para las OPKs del protocolo X3DH.
--
--  ✅ Ventajas sobre JSONB:
--    - DELETE de una sola fila (no rewrite de toda la fila de user_keys)
--    - Lock mínimo: solo la fila de la OPK a consumir
--    - Indexable: se puede buscar por user_id + is_consumed eficientemente
--    - Auditable: se puede ver historial de cuántas se consumieron
--    - Escalable: millones de OPKs sin problema
--
--  Flujo de consumo:
--    1. SELECT ... FOR UPDATE SKIP LOCKED (evita contención)
--    2. UPDATE is_consumed = TRUE, consumed_at = NOW()
--    3. Retornar al cliente la clave
--    4. Job de limpieza borra las consumidas periódicamente
--
--  is_consumed en lugar de DELETE inmediato: permite auditoría
--  y evita gaps en contadores. El job de limpieza borra en batch.
-- ============================================================

CREATE TABLE one_time_prekeys (
                                  id           BIGSERIAL   PRIMARY KEY,                         -- BIGSERIAL: más eficiente que UUID para pk de tabla de alta escritura
                                  user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                  key_id       INTEGER     NOT NULL,                            -- ID asignado por el cliente
                                  public_key   TEXT        NOT NULL,                            -- clave pública OPK (base64)
                                  is_consumed  BOOLEAN     NOT NULL DEFAULT FALSE,
                                  consumed_at  TIMESTAMPTZ,
                                  created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                  UNIQUE (user_id, key_id)
);

-- Índice parcial crítico: solo OPKs disponibles, por usuario
-- Este es el índice que usa la función get_user_public_keys en cada mensaje nuevo
CREATE INDEX idx_opk_available ON one_time_prekeys (user_id, id) WHERE is_consumed = FALSE;
-- Para el job de limpieza
CREATE INDEX idx_opk_consumed   ON one_time_prekeys (consumed_at) WHERE is_consumed = TRUE;

-- ============================================================
--  USER_SESSIONS
--  Un usuario puede tener múltiples sesiones (multi-device).
--  Permite revocar sesiones individualmente (como Telegram).
--  push_token: FCM/APNs para notificaciones push.
-- ============================================================

CREATE TABLE user_sessions (
                               id              UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                               user_id         UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                               device_id       TEXT        NOT NULL,                         -- UUID generado en el dispositivo
                               device_name     TEXT        NOT NULL DEFAULT '',              -- "iPhone de Juan", "Chrome en Mac"
                               device_type     device_type NOT NULL,
                               push_token      TEXT,                                         -- FCM o APNs token
                               ip_address      INET,                                         -- tipo nativo de Postgres para IPs
                               user_agent      TEXT,
                               last_active_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               expires_at      TIMESTAMPTZ NOT NULL,                         -- JWT/refresh token expiry
                               created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               UNIQUE (user_id, device_id)
);

CREATE INDEX idx_sessions_user       ON user_sessions (user_id);
CREATE INDEX idx_sessions_expires    ON user_sessions (expires_at);
CREATE INDEX idx_sessions_device     ON user_sessions (device_id);

-- ============================================================
--  CONTACTS
--  Lista de contactos de cada usuario.
--  contact_id puede ser NULL si el número no está registrado,
--  para permitir mostrar "Invitar a Juan" en la UI.
-- ============================================================

-- ✅ Fix: función de normalización E.164 marcada como VOLATILE (no IMMUTABLE).
-- IMMUTABLE le dice a Postgres que para el mismo input siempre retorna el mismo
-- output Y que puede cachear el resultado e incluso usarlo en índices funcionales.
-- normalize_e164 NO cumple los contratos de IMMUTABLE porque:
--   1. Lanza excepciones (efecto secundario observable)
--   2. regexp_replace con 'g' depende de la configuración de locale
--   3. El estándar prohíbe RAISE en IMMUTABLE (puede causar comportamiento
--      indefinido si Postgres intenta evaluar la función durante planning)
-- VOLATILE es correcto: sin caché, sin precondiciones, comportamiento predecible.
CREATE OR REPLACE FUNCTION normalize_e164(phone TEXT)
    RETURNS TEXT AS $$
DECLARE
    cleaned TEXT;
BEGIN
    -- Eliminar todo excepto dígitos y el '+' inicial
    cleaned := regexp_replace(phone, '[^\d+]', '', 'g');
    -- Asegurar que empiece con '+'
    IF NOT cleaned ~ '^\+' THEN
        RAISE EXCEPTION 'Número de teléfono debe incluir código de país con + (recibido: %)', phone
            USING ERRCODE = 'invalid_parameter_value';
    END IF;
    -- Validar longitud E.164: + seguido de 7 a 15 dígitos
    IF NOT cleaned ~ '^\+\d{7,15}$' THEN
        RAISE EXCEPTION 'Número de teléfono inválido (formato E.164 requerido): %', phone
            USING ERRCODE = 'invalid_parameter_value';
    END IF;
    RETURN cleaned;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- Trigger para normalizar phone en users antes de INSERT/UPDATE
CREATE OR REPLACE FUNCTION trg_normalize_user_phone()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.phone := normalize_e164(NEW.phone);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER users_normalize_phone
    BEFORE INSERT OR UPDATE OF phone ON users
    FOR EACH ROW EXECUTE FUNCTION trg_normalize_user_phone();

-- Trigger para normalizar phone en contacts antes de INSERT/UPDATE
CREATE OR REPLACE FUNCTION trg_normalize_contact_phone()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.phone := normalize_e164(NEW.phone);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TABLE contacts (
                          id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                          owner_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                          contact_id   UUID        REFERENCES users(id) ON DELETE SET NULL,  -- NULL si no está registrado
                          phone        TEXT        NOT NULL,                                  -- almacenado normalizado E.164
                          nickname     TEXT,                                                  -- alias local, no público
                          is_favorite  BOOLEAN     NOT NULL DEFAULT FALSE,
                          created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                          UNIQUE (owner_id, phone),
    -- ✅ Constraint de formato E.164 como red de seguridad adicional al trigger
                          CONSTRAINT chk_contact_phone_e164
                              CHECK (phone ~ '^\+\d{7,15}$')
);

CREATE TRIGGER contacts_normalize_phone
    BEFORE INSERT OR UPDATE OF phone ON contacts
    FOR EACH ROW EXECUTE FUNCTION trg_normalize_contact_phone();

CREATE INDEX idx_contacts_owner   ON contacts (owner_id);
CREATE INDEX idx_contacts_contact ON contacts (contact_id) WHERE contact_id IS NOT NULL;

-- ============================================================
--  USER_BLOCKS
--  Cuando A bloquea a B:
--  - B no puede enviar mensajes a A
--  - A no aparece en las búsquedas de B
--  - B no ve el avatar ni el estado de A
-- ============================================================

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

-- ============================================================
--  CHATS
--  'private': chat 1 a 1. name y avatar_url toman los del otro usuario en la app.
--  'group':   grupo tradicional con múltiples participantes.
--  'channel': canal de broadcast (solo admins envían), como Telegram channels.
--  'self':    chat personal "Mensajes guardados" (como en Telegram).
--
--  invite_link: slug único para unirse a grupos/canales públicos.
--  is_encrypted: indica si el chat usa E2E (siempre true en privados).
-- ============================================================

CREATE TABLE chats (
                       id            UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                       type          chat_type   NOT NULL,
                       name          TEXT,                                           -- solo para group/channel
                       description   TEXT,
                       avatar_url    TEXT,
                       created_by    UUID        REFERENCES users(id) ON DELETE SET NULL,
                       invite_link   TEXT        UNIQUE,                            -- slug para grupos públicos
                       is_encrypted  BOOLEAN     NOT NULL DEFAULT TRUE,
                       max_members   INTEGER,                                       -- NULL = sin límite

    -- ✅ Fix: chats privados no tienen nombre ni invite_link
    -- Los canales/grupos sí deben tener nombre obligatorio
                       CONSTRAINT chk_group_has_name
                           CHECK (type NOT IN ('group', 'channel') OR (name IS NOT NULL AND name <> '')),
                       CONSTRAINT chk_private_no_invite
                           CHECK (type != 'private' OR invite_link IS NULL),
    -- max_members solo aplica en grupos/canales
                       CONSTRAINT chk_max_members_scope
                           CHECK (type IN ('group', 'channel') OR max_members IS NULL),

                       created_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                       updated_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                       deleted_at    TIMESTAMPTZ
);

CREATE TRIGGER chats_updated_at
    BEFORE UPDATE ON chats
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

CREATE INDEX idx_chats_invite_link ON chats (invite_link) WHERE invite_link IS NOT NULL;
CREATE INDEX idx_chats_created_by  ON chats (created_by);
-- NOTA: el trigger trg_enforce_private_chat_limit se declara justo después
-- de CREATE TABLE chat_participants, que es la tabla que referencia.

-- ============================================================
--  CHAT_SETTINGS
--  Configuraciones POR USUARIO para cada chat.
--  Separado de chat_participants para poder hacer queries
--  de "chats silenciados" sin traer datos de participación.
-- ============================================================

CREATE TABLE chat_settings (
                               user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                               chat_id      UUID        NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
                               is_muted     BOOLEAN     NOT NULL DEFAULT FALSE,
                               muted_until  TIMESTAMPTZ,                                    -- NULL = silenciado indefinidamente
                               is_pinned    BOOLEAN     NOT NULL DEFAULT FALSE,
                               pin_order    INTEGER     NOT NULL DEFAULT 0,                 -- orden entre chats pineados
                               is_archived  BOOLEAN     NOT NULL DEFAULT FALSE,
                               theme        TEXT,                                           -- color/wallpaper personalizado
                               updated_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                               PRIMARY KEY (user_id, chat_id)
);

CREATE TRIGGER chat_settings_updated_at
    BEFORE UPDATE ON chat_settings
    FOR EACH ROW EXECUTE FUNCTION trigger_set_updated_at();

CREATE INDEX idx_chat_settings_pinned ON chat_settings (user_id, pin_order) WHERE is_pinned = TRUE;

-- ============================================================
--  CHAT_PARTICIPANTS
--  encryption_key_enc: la clave de sesión del grupo, cifrada
--  con la clave pública de ESTE participante. Cada participante
--  tiene su propia copia cifrada de la misma clave simétrica.
--  Esto es el patrón de "sealed sender" / group key distribution.
--
--  added_by: auditoría de quién añadió a quién al grupo.
-- ============================================================

CREATE TABLE chat_participants (
                                   id                  UUID              PRIMARY KEY DEFAULT gen_random_uuid(),
                                   chat_id             UUID              NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
                                   user_id             UUID              NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                   role                participant_role  NOT NULL DEFAULT 'member',
                                   encryption_key_enc  TEXT,                                    -- clave de grupo cifrada para este user
                                   added_by            UUID              REFERENCES users(id) ON DELETE SET NULL,
                                   joined_at           TIMESTAMPTZ       NOT NULL DEFAULT NOW(),
                                   left_at             TIMESTAMPTZ,                             -- NULL = sigue en el grupo
                                   disappearing_ttl    INTEGER,                                 -- override del TTL global del chat
                                   UNIQUE (chat_id, user_id)
);

CREATE INDEX idx_participants_chat    ON chat_participants (chat_id) WHERE left_at IS NULL;
CREATE INDEX idx_participants_user    ON chat_participants (user_id) WHERE left_at IS NULL;
-- Índice parcial para buscar admins/owners de un grupo rápido
CREATE INDEX idx_participants_admins  ON chat_participants (chat_id, role) WHERE role IN ('owner', 'admin') AND left_at IS NULL;

-- Trigger: impide añadir un 3er participante a un chat privado.
-- Declarado aquí (después de CREATE TABLE chat_participants) para que
-- el trigger pueda hacer COUNT(*) sobre la tabla que ya existe.
-- La función referencia chat_participants, por eso no puede ir antes.
CREATE OR REPLACE FUNCTION enforce_private_chat_limit()
    RETURNS TRIGGER AS $$
DECLARE
    chat_kind         chat_type;
    participant_count INTEGER;
BEGIN
    SELECT type INTO chat_kind FROM chats WHERE id = NEW.chat_id;

    IF chat_kind = 'private' THEN
        SELECT COUNT(*) INTO participant_count
        FROM chat_participants
        WHERE chat_id = NEW.chat_id AND left_at IS NULL;

        IF participant_count >= 2 THEN
            RAISE EXCEPTION 'Un chat privado no puede tener más de 2 participantes activos (chat_id: %)', NEW.chat_id
                USING ERRCODE = 'check_violation';
        END IF;
    END IF;

    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trg_enforce_private_chat_limit
    BEFORE INSERT ON chat_participants
    FOR EACH ROW EXECUTE FUNCTION enforce_private_chat_limit();

-- ============================================================
--  MESSAGES
--  content_encrypted: texto cifrado (AES-GCM), en base64.
--  content_iv:        vector de inicialización para AES-GCM.
--  metadata:          datos del adjunto o ubicación (sin PII).
--  reply_to_id:       referencia circular a messages (respuestas).
--  expires_at:        para mensajes que se autodestruyen.
--  is_forwarded:      flag para mostrar "reenviado" en la UI.
-- ============================================================

CREATE TABLE messages (
                          id                UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
                          chat_id           UUID          NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
                          sender_id         UUID          REFERENCES users(id) ON DELETE SET NULL,   -- NULL = usuario eliminado
                          reply_to_id       UUID          REFERENCES messages(id) ON DELETE SET NULL,
                          content_encrypted TEXT,                                      -- NULL si es solo adjunto/sistema
                          content_iv        TEXT,                                      -- IV para AES-GCM
                          message_type      message_type  NOT NULL DEFAULT 'text',
                          metadata          JSONB,                                     -- {width, height, duration, size, mime_type...}
                          is_forwarded      BOOLEAN       NOT NULL DEFAULT FALSE,
                          created_at        TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
                          edited_at         TIMESTAMPTZ,
                          deleted_at        TIMESTAMPTZ,                               -- soft delete
                          expires_at        TIMESTAMPTZ,                               -- mensajes temporales

    -- ✅ Fix: evitar mensajes vacíos inválidos
    -- Un mensaje de tipo 'text' siempre debe tener contenido cifrado.
    -- Un mensaje de tipo adjunto/sistema puede no tener texto (content_encrypted NULL).
    -- Un mensaje de tipo 'deleted' no tiene contenido (se borró).
    -- content_iv debe existir si y solo si content_encrypted existe.
                          CONSTRAINT chk_text_message_has_content
                              CHECK (
                                  message_type != 'text'
                                      OR deleted_at IS NOT NULL
                                      OR (content_encrypted IS NOT NULL AND content_encrypted <> '' AND content_iv IS NOT NULL)
                                  ),
                          CONSTRAINT chk_content_iv_consistency
                              CHECK (
                                  (content_encrypted IS NULL) = (content_iv IS NULL)
                                  )
);

-- Índice compuesto principal: listar mensajes de un chat paginados por fecha
CREATE INDEX idx_messages_chat_created    ON messages (chat_id, created_at DESC) WHERE deleted_at IS NULL;
-- ✅ Fix: índice simple en chat_id para queries sin ORDER BY (COUNT, EXISTS, joins).
-- El índice compuesto (chat_id, created_at DESC) NO sirve cuando el planner
-- necesita solo filtrar por chat_id sin ordenar: el costo de leer el índice
-- completo supera al de un índice simple. Tener ambos le da al planner opciones.
-- Ejemplo de queries que usan este índice: COUNT(*) de mensajes en un chat,
-- EXISTS para saber si un chat tiene mensajes, JOIN desde chat_participants.
CREATE INDEX idx_messages_chat_id         ON messages (chat_id) WHERE deleted_at IS NULL;
-- ✅ Fix anterior: índice en sender_id
CREATE INDEX idx_messages_sender          ON messages (sender_id, created_at DESC) WHERE deleted_at IS NULL AND sender_id IS NOT NULL;
-- Para expiración: job de limpieza necesita este índice
CREATE INDEX idx_messages_expires         ON messages (expires_at) WHERE expires_at IS NOT NULL AND deleted_at IS NULL;
-- Para buscar replies
CREATE INDEX idx_messages_reply_to        ON messages (reply_to_id) WHERE reply_to_id IS NOT NULL;

-- ============================================================
--  MESSAGE_ATTACHMENTS
--  Separado de messages para:
--  1. Un mensaje puede tener múltiples adjuntos (álbum de fotos).
--  2. Facilita manejo de lifecycle de archivos en storage (S3, etc.).
--
--  encryption_key_enc: clave AES del archivo, cifrada con la
--  clave pública del receptor. El servidor nunca ve el archivo en claro.
-- ============================================================

CREATE TABLE message_attachments (
                                     id                   UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                     message_id           UUID        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                                     file_url             TEXT        NOT NULL,                   -- URL en storage (S3/R2/etc.)
                                     file_type            TEXT        NOT NULL,                   -- MIME type: image/jpeg, video/mp4...
                                     file_size            BIGINT      NOT NULL,                   -- bytes
                                     file_name            TEXT,                                   -- nombre original del archivo
                                     thumbnail_url        TEXT,                                   -- preview para video/imagen
                                     width                INTEGER,                                -- píxeles
                                     height               INTEGER,
                                     duration_seconds     INTEGER,                                -- para audio/video
                                     encryption_key_enc   TEXT,                                   -- clave del archivo cifrada para receptor
                                     encryption_iv        TEXT,
                                     created_at           TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_attachments_message ON message_attachments (message_id);

-- ============================================================
--  MESSAGE_REACTIONS
--  UNIQUE (message_id, user_id, reaction) permite reacciones
--  múltiples distintas, pero no duplicar la misma reacción.
--  Si quieres solo UNA reacción por usuario, cambia a
--  UNIQUE (message_id, user_id).
-- ============================================================

CREATE TABLE message_reactions (
                                   id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                                   message_id   UUID        NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                                   user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                   reaction     TEXT        NOT NULL,                           -- emoji unicode: 👍 ❤️ 😂 😮 😢 🙏
                                   created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                                   UNIQUE (message_id, user_id, reaction)
);

CREATE INDEX idx_reactions_message ON message_reactions (message_id);
CREATE INDEX idx_reactions_user    ON message_reactions (user_id);

-- ============================================================
--  MESSAGE_STATUS
--  Tracking por destinatario:
--  - En chat privado: 1 fila por mensaje (el receptor).
--  - En grupo: 1 fila por miembro del grupo.
--  El "tick doble azul" se calcula en app: todos los miembros en 'read'.
--
--  NOTA: En grupos grandes esto puede generar MUCHO volumen.
--  Para grupos >500 personas considera una estrategia diferente
--  (solo track de primeros N lectores, o solo "delivered" count).
-- ============================================================

CREATE TABLE message_status (
                                message_id   UUID           NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                                user_id      UUID           NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                status       message_status_enum  NOT NULL DEFAULT 'sent',
                                updated_at   TIMESTAMPTZ    NOT NULL DEFAULT NOW(),
                                PRIMARY KEY (message_id, user_id)
);

CREATE INDEX idx_msg_status_user    ON message_status (user_id, updated_at DESC);
CREATE INDEX idx_msg_status_message ON message_status (message_id, status);

-- ============================================================
--  STORIES (Estados)
--  expires_at: por defecto 24h después de created_at.
--  privacy: quién puede ver este estado.
--  'contacts_except' y 'selected' requieren tabla de excepciones.
-- ============================================================

CREATE TABLE stories (
                         id            UUID          PRIMARY KEY DEFAULT gen_random_uuid(),
                         user_id       UUID          NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                         content_url   TEXT          NOT NULL,
                         content_type  TEXT          NOT NULL DEFAULT 'image',        -- 'image' | 'video'
                         caption       TEXT,
                         privacy       story_privacy NOT NULL DEFAULT 'contacts',
                         created_at    TIMESTAMPTZ   NOT NULL DEFAULT NOW(),
                         expires_at    TIMESTAMPTZ   NOT NULL DEFAULT (NOW() + INTERVAL '24 hours'),
                         deleted_at    TIMESTAMPTZ
);

CREATE INDEX idx_stories_user       ON stories (user_id, created_at DESC) WHERE deleted_at IS NULL;
CREATE INDEX idx_stories_active     ON stories (expires_at)               WHERE deleted_at IS NULL;

-- ============================================================
--  STORY_PRIVACY_EXCEPTIONS
--  Para los modos 'contacts_except' y 'selected'.
--  is_excluded = TRUE: excluir de 'contacts_except'
--  is_excluded = FALSE: incluir en 'selected'
-- ============================================================

CREATE TABLE story_privacy_exceptions (
                                          story_id     UUID        NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
                                          user_id      UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                                          is_excluded  BOOLEAN     NOT NULL DEFAULT FALSE,
                                          PRIMARY KEY (story_id, user_id)
);

-- ============================================================
--  STORY_VIEWS
--  Registro de quién vio qué estado.
--  reaction: permite reaccionar a un estado (como WhatsApp).
-- ============================================================

CREATE TABLE story_views (
                             id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
                             story_id     UUID        NOT NULL REFERENCES stories(id) ON DELETE CASCADE,
                             viewer_id    UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                             reaction     TEXT,                                           -- emoji de reacción al estado
                             viewed_at    TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                             UNIQUE (story_id, viewer_id)
);

CREATE INDEX idx_story_views_story  ON story_views (story_id, viewed_at DESC);
CREATE INDEX idx_story_views_viewer ON story_views (viewer_id);

-- ============================================================
--  NOTIFICATIONS
--  data JSONB: payload flexible según el tipo.
--  Ejemplo para 'message': {chat_id, message_id, sender_name}
--  Ejemplo para 'group_invite': {chat_id, chat_name, invited_by}
-- ============================================================

CREATE TABLE notifications (
                               id           UUID               PRIMARY KEY DEFAULT gen_random_uuid(),
                               user_id      UUID               NOT NULL REFERENCES users(id) ON DELETE CASCADE,
                               type         notification_type  NOT NULL,
                               data         JSONB              NOT NULL DEFAULT '{}',
                               is_read      BOOLEAN            NOT NULL DEFAULT FALSE,
                               read_at      TIMESTAMPTZ,
                               created_at   TIMESTAMPTZ        NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_notifications_user      ON notifications (user_id, created_at DESC) WHERE is_read = FALSE;
CREATE INDEX idx_notifications_created   ON notifications (created_at);              -- para limpieza

-- ============================================================
--  AUDIT_LOG
--  Inmutable por diseño: NO UPDATE, NO DELETE en producción.
--  Registra acciones sensibles para compliance y forense.
--  Usa particionamiento por mes para gestionar el volumen.
-- ============================================================

CREATE TABLE audit_log (
    id          UUID        DEFAULT gen_random_uuid(),
    user_id     UUID,
    action      audit_action NOT NULL,
    entity_type TEXT,
    entity_id   UUID,
    metadata    JSONB       NOT NULL DEFAULT '{}',
    ip_address  INET,
    user_agent  TEXT,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (id, created_at)  
) PARTITION BY RANGE (created_at);

-- ============================================================
--  PARTICIONES DE AUDIT_LOG
--
--  ✅ Fix: en lugar de 3 particiones hardcodeadas (que generan
--  ERROR en producción en cuanto cambia el mes), se usa una función
--  que crea particiones de forma automática para N meses.
--
--  Estrategia de operación:
--  1. Esta función crea particiones desde hoy hasta 3 meses adelante.
--  2. Un job de pg_cron la llama el día 1 de cada mes:
--       SELECT cron.schedule('create-audit-partitions', '0 0 1 * *',
--         $$ SELECT create_audit_log_partitions(3); $$);
--  3. Si no tienes pg_cron: llamarla desde tu proceso de deploy.
--
--  La opción robusta a largo plazo es pg_partman, que gestiona
--  esto automáticamente con retención configurable.
-- ============================================================

CREATE OR REPLACE FUNCTION create_audit_log_partitions(months_ahead INTEGER DEFAULT 3)
    RETURNS void AS $$
DECLARE
    start_date  DATE;
    end_date    DATE;
    part_name   TEXT;
    cur_month   DATE := DATE_TRUNC('month', NOW())::DATE;
BEGIN
    FOR i IN 0..months_ahead LOOP
            start_date := cur_month + (i * INTERVAL '1 month');
            end_date   := start_date + INTERVAL '1 month';
            part_name  := 'audit_log_' || TO_CHAR(start_date, 'YYYY_MM');

            -- Solo crear si no existe
            IF NOT EXISTS (
                SELECT 1 FROM pg_class c
                                  JOIN pg_namespace n ON n.oid = c.relnamespace
                WHERE c.relname = part_name AND n.nspname = current_schema()
            ) THEN
                EXECUTE format(
                        'CREATE TABLE %I PARTITION OF audit_log FOR VALUES FROM (%L) TO (%L)',
                        part_name, start_date, end_date
                        );
                RAISE NOTICE 'Partición creada: %', part_name;
            END IF;
        END LOOP;
END;
$$ LANGUAGE plpgsql;

-- Crear particiones desde el inicio del historial hasta 3 meses adelante
-- Ajusta el rango histórico según tu fecha de inicio real de producción
DO $$
    DECLARE
        start_date DATE := '2025-01-01';
        end_date   DATE;
        part_name  TEXT;
        cur        DATE := start_date;
        -- Crear hasta 3 meses en el futuro desde hoy
        limit_date DATE := DATE_TRUNC('month', NOW() + INTERVAL '3 months')::DATE + INTERVAL '1 month';
    BEGIN
        WHILE cur < limit_date LOOP
                end_date  := cur + INTERVAL '1 month';
                part_name := 'audit_log_' || TO_CHAR(cur, 'YYYY_MM');

                IF NOT EXISTS (
                    SELECT 1 FROM pg_class c
                                      JOIN pg_namespace n ON n.oid = c.relnamespace
                    WHERE c.relname = part_name AND n.nspname = current_schema()
                ) THEN
                    EXECUTE format(
                            'CREATE TABLE %I PARTITION OF audit_log FOR VALUES FROM (%L) TO (%L)',
                            part_name, cur, end_date
                            );
                END IF;

                cur := end_date;
            END LOOP;
    END;
$$;

CREATE INDEX idx_audit_user    ON audit_log (user_id, created_at DESC);
CREATE INDEX idx_audit_action  ON audit_log (action, created_at DESC);
CREATE INDEX idx_audit_entity  ON audit_log (entity_type, entity_id);

-- ============================================================
--  ROW-LEVEL SECURITY (RLS)
--  CRÍTICO para seguridad: que ningún usuario pueda leer
--  datos de otros usuarios, incluso si hay un bug en la app.
--  Activar por tabla y definir políticas.
--
--  Nota: RLS tiene costo de rendimiento (~5-15%). Para APIs
--  internas donde el backend ya filtra correctamente,
--  puede usarse solo en tablas más sensibles.
-- ============================================================

-- Ejemplo de RLS en messages:
ALTER TABLE messages ENABLE ROW LEVEL SECURITY;

-- El usuario solo puede ver mensajes de chats en los que participa.
-- ✅ Fix: current_setting('app.current_user_id', true) — el segundo argumento
-- `true` es missing_ok. Sin él, si la variable no está seteada (conexión de
-- un worker interno, migration runner, backup, etc.) Postgres lanza:
--   ERROR: unrecognized configuration parameter "app.current_user_id"
-- Con missing_ok=true retorna NULL, el CAST a UUID retorna NULL,
-- y la policy deniega el acceso silenciosamente — que es el comportamiento correcto.
CREATE POLICY messages_select ON messages
    FOR SELECT
    USING (
    chat_id IN (
        SELECT chat_id FROM chat_participants
        WHERE user_id = current_setting('app.current_user_id', true)::UUID
          AND left_at IS NULL
    )
    );

-- Solo el sender puede editar/borrar sus mensajes
CREATE POLICY messages_update ON messages
    FOR UPDATE
    USING (sender_id = current_setting('app.current_user_id', true)::UUID);

-- Ejemplo en user_sessions: solo ver tus propias sesiones
ALTER TABLE user_sessions ENABLE ROW LEVEL SECURITY;

CREATE POLICY sessions_select ON user_sessions
    FOR SELECT
    USING (user_id = current_setting('app.current_user_id', true)::UUID);

-- ============================================================
--  VISTAS ÚTILES
-- ============================================================

-- Vista: lista de chats con último mensaje (para la pantalla principal)
CREATE VIEW v_chat_previews AS
SELECT
    cp.user_id,
    c.id         AS chat_id,
    c.type,
    c.name,
    c.avatar_url,
    m.id         AS last_message_id,
    m.message_type,
    m.content_encrypted AS last_message_encrypted,
    m.sender_id  AS last_sender_id,
    m.created_at AS last_message_at,
    cs.is_pinned,
    cs.pin_order,
    cs.is_muted,
    cs.is_archived
FROM chat_participants cp
         JOIN chats c             ON c.id = cp.chat_id AND c.deleted_at IS NULL
         LEFT JOIN LATERAL (
    SELECT id, message_type, content_encrypted, sender_id, created_at
    FROM messages
    WHERE chat_id = c.id AND deleted_at IS NULL
    ORDER BY created_at DESC
    LIMIT 1
    ) m ON TRUE
         LEFT JOIN chat_settings cs ON cs.chat_id = c.id AND cs.user_id = cp.user_id
WHERE cp.left_at IS NULL;

-- Vista: estados activos visibles (no expirados ni eliminados)
CREATE VIEW v_active_stories AS
SELECT s.*, u.username, up.display_name, u.avatar_url
FROM stories s
         JOIN users u         ON u.id = s.user_id AND u.deleted_at IS NULL
         JOIN user_profiles up ON up.user_id = s.user_id
WHERE s.expires_at > NOW()
  AND s.deleted_at IS NULL;

-- ============================================================
--  FUNCIONES DE UTILIDAD
-- ============================================================

-- Función: obtener la clave pública de un usuario para cifrado
-- El cliente la llama antes de enviar un mensaje privado.
-- ✅ Fix: usa la tabla one_time_prekeys con FOR UPDATE SKIP LOCKED
-- en lugar de JSONB. SKIP LOCKED evita contención si dos dispositivos
-- intentan establecer sesión con el mismo usuario simultáneamente.
CREATE OR REPLACE FUNCTION get_user_public_keys(target_user_id UUID)
    RETURNS TABLE (
                      identity_key       TEXT,
                      signed_prekey      TEXT,
                      signed_prekey_id   INTEGER,
                      signed_prekey_sig  TEXT,
                      one_time_prekey_id INTEGER,
                      one_time_prekey    TEXT
                  ) AS $$
DECLARE
    v_opk_id     BIGINT;
    v_opk_key_id INTEGER;
    v_opk_key    TEXT;
BEGIN
    -- Seleccionar y marcar como consumida la primera OPK disponible.
    -- SKIP LOCKED: si otra transacción ya está procesando esta fila,
    -- saltarla y tomar la siguiente. Evita deadlocks en alta concurrencia.
    SELECT id, key_id, public_key
    INTO v_opk_id, v_opk_key_id, v_opk_key
    FROM one_time_prekeys
    WHERE user_id = target_user_id
      AND is_consumed = FALSE
    ORDER BY id
    LIMIT 1
        FOR UPDATE SKIP LOCKED;

    IF v_opk_id IS NOT NULL THEN
        -- Marcar como consumida (no DELETE: permite auditoría y batch cleanup)
        UPDATE one_time_prekeys
        SET is_consumed = TRUE, consumed_at = NOW()
        WHERE id = v_opk_id;

        -- Decrementar contador en user_keys
        UPDATE user_keys
        SET prekey_count = GREATEST(prekey_count - 1, 0),
            updated_at   = NOW()
        WHERE user_id = target_user_id;
    END IF;
    -- Si no hay OPKs disponibles: se retorna NULL en one_time_prekey_id y one_time_prekey.
    -- El protocolo X3DH permite sesión sin OPK (menor forward secrecy, pero funcional).
    -- La app debe alertar al usuario para que suba más OPKs.

    RETURN QUERY
        SELECT
            uk.identity_key,
            uk.signed_prekey,
            uk.signed_prekey_id,
            uk.signed_prekey_sig,
            v_opk_key_id,
            v_opk_key
        FROM user_keys uk
        WHERE uk.user_id = target_user_id;
END;
$$ LANGUAGE plpgsql;

-- Función: marcar mensajes como leídos en batch
CREATE OR REPLACE FUNCTION mark_messages_read(
    p_user_id UUID,
    p_chat_id UUID,
    p_up_to   TIMESTAMPTZ DEFAULT NOW()
)
    RETURNS INTEGER AS $$
DECLARE
    updated_count INTEGER;
BEGIN
    UPDATE message_status ms
    SET status = 'read', updated_at = NOW()
    FROM messages m
    WHERE ms.message_id = m.id
      AND ms.user_id = p_user_id
      AND m.chat_id = p_chat_id
      AND m.created_at <= p_up_to
      AND ms.status <> 'read'
      AND m.deleted_at IS NULL;

    GET DIAGNOSTICS updated_count = ROW_COUNT;
    RETURN updated_count;
END;
$$ LANGUAGE plpgsql;

