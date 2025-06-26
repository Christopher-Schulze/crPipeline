ALTER TABLE users
ADD COLUMN is_active BOOLEAN NOT NULL DEFAULT true,
ADD COLUMN deactivated_at TIMESTAMP WITH TIME ZONE;

-- Optionally, add an index on is_active if frequently queried
CREATE INDEX IF NOT EXISTS idx_users_is_active ON users(is_active);
