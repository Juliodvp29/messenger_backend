-- 007_contacts.sql
-- ============================================================
-- HELPER: E.164 Normalization
CREATE OR REPLACE FUNCTION normalize_e164(phone TEXT)
    RETURNS TEXT AS $$
DECLARE
    cleaned TEXT;
BEGIN
    cleaned := regexp_replace(phone, '[^\d+]', '', 'g');
    IF NOT cleaned ~ '^\+' THEN
        RAISE EXCEPTION 'Número de teléfono debe incluir código de país con + (recibido: %)', phone
            USING ERRCODE = 'invalid_parameter_value';
    END IF;
    IF NOT cleaned ~ '^\+\d{7,15}$' THEN
        RAISE EXCEPTION 'Número de teléfono inválido (formato E.164 requerido): %', phone
            USING ERRCODE = 'invalid_parameter_value';
    END IF;
    RETURN cleaned;
END;
$$ LANGUAGE plpgsql VOLATILE;

-- Normalization Trigger for Users
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

-- CONTACTS
CREATE TABLE contacts (
    id           UUID        PRIMARY KEY DEFAULT gen_random_uuid(),
    owner_id     UUID        NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    contact_id   UUID        REFERENCES users(id) ON DELETE SET NULL,
    phone        TEXT        NOT NULL,
    nickname     TEXT,
    is_favorite  BOOLEAN     NOT NULL DEFAULT FALSE,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (owner_id, phone),
    CONSTRAINT chk_contact_phone_e164 CHECK (phone ~ '^\+\d{7,15}$')
);

CREATE OR REPLACE FUNCTION trg_normalize_contact_phone()
    RETURNS TRIGGER AS $$
BEGIN
    NEW.phone := normalize_e164(NEW.phone);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER contacts_normalize_phone
    BEFORE INSERT OR UPDATE OF phone ON contacts
    FOR EACH ROW EXECUTE FUNCTION trg_normalize_contact_phone();

CREATE INDEX idx_contacts_owner   ON contacts (owner_id);
CREATE INDEX idx_contacts_contact ON contacts (contact_id) WHERE contact_id IS NOT NULL;
