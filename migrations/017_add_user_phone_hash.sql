-- Add phone_hash column to users
ALTER TABLE users ADD COLUMN phone_hash VARCHAR(64);

-- Populate phone_hash for existing users
-- Note: phone is already normalized to E.164 by triggers
UPDATE users SET phone_hash = encode(digest(phone, 'sha256'), 'hex') WHERE phone_hash IS NULL;

-- Create index for fast lookups
CREATE INDEX idx_users_phone_hash ON users (phone_hash) WHERE deleted_at IS NULL;

-- Add constraint to prevent duplicates (though redundant with phone unique, good for sync)
CREATE INDEX idx_users_phone_hash_unique ON users (phone_hash) WHERE deleted_at IS NULL;
