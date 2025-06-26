ALTER TABLE users
ADD COLUMN created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW();

-- For existing users, created_at will be set to NOW() when this migration runs.
-- If a more accurate historical created_at is needed and available from other sources (e.g., audit logs),
-- a more complex backfill would be required. For now, NOW() for old records is acceptable.
