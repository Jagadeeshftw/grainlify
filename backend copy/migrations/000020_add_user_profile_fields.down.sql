-- Remove profile fields from users table
ALTER TABLE users
  DROP COLUMN IF EXISTS first_name,
  DROP COLUMN IF EXISTS last_name,
  DROP COLUMN IF EXISTS location,
  DROP COLUMN IF EXISTS website,
  DROP COLUMN IF EXISTS bio,
  DROP COLUMN IF EXISTS avatar_url;

-- Drop indexes
DROP INDEX IF EXISTS idx_users_first_name;
DROP INDEX IF EXISTS idx_users_last_name;

