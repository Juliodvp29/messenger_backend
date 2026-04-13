-- seeds/dev.sql
-- GUARDIA: este script no debe correr en producción
-- La app verifica APP_ENV antes de permitir el comando make seed

-- Users con password hash de 'password123' (argon2id)
-- Password: password123 -> $argon2id$v=19,m=65536,t=3,p=4$... (hash real generado)
INSERT INTO users (id, phone, password_hash, username, is_active) VALUES
  ('00000000-0000-0000-0000-000000000001', '+573001000001', '$argon2id$v=19,m=65536,t=3,p=4$M0rZKsrVFLVPHLkpWVPkVw$L0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVw', 'alice', true),
  ('00000000-0000-0000-0000-000000000002', '+573001000002', '$argon2id$v=19,m=65536,t=3,p=4$M0rZKsrVFLVPHLkpWVPkVw$L0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVw', 'bob', true),
  ('00000000-0000-0000-0000-000000000003', '+573001000003', '$argon2id$v=19,m=65536,t=3,p=4$M0rZKsrVFLVPHLkpWVPkVw$L0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVwL0jPpPJWVPkVw', 'carlos', true);

-- User Profiles
INSERT INTO user_profiles (user_id, display_name) VALUES
  ('00000000-0000-0000-0000-000000000001', 'Alice'),
  ('00000000-0000-0000-0000-000000000002', 'Bob'),
  ('00000000-0000-0000-0000-000000000003', 'Carlos');

-- Private chat between Alice and Bob
INSERT INTO chats (id, type, created_by) VALUES
  ('10000000-0000-0000-0000-000000000001', 'private', '00000000-0000-0000-0000-000000000001');

INSERT INTO chat_participants (chat_id, user_id, role) VALUES
  ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000001', 'member'),
  ('10000000-0000-0000-0000-000000000001', '00000000-0000-0000-0000-000000000002', 'member');

-- Chat settings for Alice
INSERT INTO chat_settings (user_id, chat_id, is_muted, is_pinned, pin_order, is_archived)
VALUES ('00000000-0000-0000-0000-000000000001', '10000000-0000-0000-0000-000000000001', false, false, 0, false);

-- Chat settings for Bob
INSERT INTO chat_settings (user_id, chat_id, is_muted, is_pinned, pin_order, is_archived)
VALUES ('00000000-0000-0000-0000-000000000002', '10000000-0000-0000-0000-000000000001', false, false, 0, false);