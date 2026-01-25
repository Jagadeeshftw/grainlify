-- Remove avatar_url column from github_accounts table
ALTER TABLE github_accounts
  DROP COLUMN IF EXISTS avatar_url;

-- Drop index
DROP INDEX IF EXISTS idx_github_accounts_user_id;


















