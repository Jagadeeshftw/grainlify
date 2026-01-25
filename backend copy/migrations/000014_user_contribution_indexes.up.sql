-- Add indexes for efficient user contribution queries
-- These indexes enable fast lookups of contributions by GitHub login (author_login)
-- which is then joined with github_accounts to get user_id

-- Index on github_issues.author_login for contribution counting
CREATE INDEX IF NOT EXISTS idx_github_issues_author_login ON github_issues(author_login) 
WHERE author_login IS NOT NULL;

-- Index on github_pull_requests.author_login for contribution counting
CREATE INDEX IF NOT EXISTS idx_github_prs_author_login ON github_pull_requests(author_login) 
WHERE author_login IS NOT NULL;

-- Index on github_accounts.login for joining with author_login
CREATE INDEX IF NOT EXISTS idx_github_accounts_login ON github_accounts(login) 
WHERE login IS NOT NULL;

-- Composite index for project filtering (only verified projects count)
CREATE INDEX IF NOT EXISTS idx_projects_status_ecosystem ON projects(status, ecosystem_id) 
WHERE status = 'verified';

-- Index for language lookups
CREATE INDEX IF NOT EXISTS idx_projects_language ON projects(language) 
WHERE language IS NOT NULL AND status = 'verified';


















