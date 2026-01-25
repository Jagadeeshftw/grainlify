-- Revert: Remove indexes for user contribution queries
DROP INDEX IF EXISTS idx_github_issues_author_login;
DROP INDEX IF EXISTS idx_github_prs_author_login;
DROP INDEX IF EXISTS idx_github_accounts_login;
DROP INDEX IF EXISTS idx_projects_status_ecosystem;
DROP INDEX IF EXISTS idx_projects_language;


















