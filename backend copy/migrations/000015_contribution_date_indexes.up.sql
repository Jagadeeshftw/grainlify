-- Add indexes for efficient date-based contribution queries
-- These indexes enable fast lookups of contributions by date for calendar/heatmap visualization

-- Index on github_issues.created_at_github for calendar queries
CREATE INDEX IF NOT EXISTS idx_github_issues_created_at ON github_issues(created_at_github) 
WHERE created_at_github IS NOT NULL AND author_login IS NOT NULL;

-- Index on github_pull_requests.created_at_github for calendar queries
CREATE INDEX IF NOT EXISTS idx_github_prs_created_at ON github_pull_requests(created_at_github) 
WHERE created_at_github IS NOT NULL AND author_login IS NOT NULL;

-- Composite index for filtering by author and date range (for calendar queries)
CREATE INDEX IF NOT EXISTS idx_github_issues_author_date ON github_issues(author_login, created_at_github) 
WHERE author_login IS NOT NULL AND created_at_github IS NOT NULL;

CREATE INDEX IF NOT EXISTS idx_github_prs_author_date ON github_pull_requests(author_login, created_at_github) 
WHERE author_login IS NOT NULL AND created_at_github IS NOT NULL;


















