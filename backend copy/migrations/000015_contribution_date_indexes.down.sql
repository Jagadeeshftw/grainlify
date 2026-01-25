-- Revert: Remove indexes for date-based contribution queries
DROP INDEX IF EXISTS idx_github_issues_created_at;
DROP INDEX IF EXISTS idx_github_prs_created_at;
DROP INDEX IF EXISTS idx_github_issues_author_date;
DROP INDEX IF EXISTS idx_github_prs_author_date;


















