-- Add profile fields to users table
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS first_name TEXT,
  ADD COLUMN IF NOT EXISTS last_name TEXT,
  ADD COLUMN IF NOT EXISTS location TEXT,
  ADD COLUMN IF NOT EXISTS website TEXT,
  ADD COLUMN IF NOT EXISTS bio TEXT,
  ADD COLUMN IF NOT EXISTS avatar_url TEXT;

-- Create indexes for faster lookups
CREATE INDEX IF NOT EXISTS idx_users_first_name ON users(first_name) WHERE first_name IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_last_name ON users(last_name) WHERE last_name IS NOT NULL;

