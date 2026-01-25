-- Add comments JSONB column to github_issues table
ALTER TABLE github_issues
  ADD COLUMN IF NOT EXISTS comments JSONB DEFAULT '[]'::jsonb;


















