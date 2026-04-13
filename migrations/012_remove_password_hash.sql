-- 012_remove_password_hash.sql
-- ============================================================
-- Eliminar columna password_hash (Fase 03: OTP + PIN local)
-- Esta migración es para bases de datos que aún tengan password_hash

-- Verificar que la columna existe antes de_droparla
DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM information_schema.columns 
        WHERE table_name = 'users' AND column_name = 'password_hash'
    ) THEN
        ALTER TABLE users DROP COLUMN IF EXISTS password_hash;
    END IF;
END $$;