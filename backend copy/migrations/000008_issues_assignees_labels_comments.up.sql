-- Add assignees, labels, and comments_count to github_issues table
ALTER TABLE github_issues
  ADD COLUMN IF NOT EXISTS assignees JSONB DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS labels JSONB DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS comments_count INT DEFAULT 0;

CREATE INDEX IF NOT EXISTS idx_github_issues_labels ON github_issues USING GIN (labels);


















