-- Add avatar_url column to github_accounts table
ALTER TABLE github_accounts
  ADD COLUMN IF NOT EXISTS avatar_url TEXT;

-- Add index for faster lookups
CREATE INDEX IF NOT EXISTS idx_github_accounts_user_id ON github_accounts(user_id);


















