-- GitHub webhook event log (auditable, deduped via delivery_id).
CREATE TABLE IF NOT EXISTS github_events (
  delivery_id TEXT PRIMARY KEY,
  project_id UUID REFERENCES projects(id) ON DELETE SET NULL,
  repo_full_name TEXT,
  event TEXT NOT NULL,
  action TEXT,
  payload JSONB NOT NULL,
  received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_github_events_project ON github_events(project_id, received_at DESC);
CREATE INDEX IF NOT EXISTS idx_github_events_repo ON github_events(repo_full_name, received_at DESC);

-- Issues (one row per GitHub issue within a registered project).
CREATE TABLE IF NOT EXISTS github_issues (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  github_issue_id BIGINT NOT NULL,
  number INT NOT NULL,
  state TEXT,
  title TEXT,
  body TEXT,
  author_login TEXT,
  url TEXT,
  created_at_github TIMESTAMPTZ,
  updated_at_github TIMESTAMPTZ,
  closed_at_github TIMESTAMPTZ,
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (project_id, github_issue_id),
  UNIQUE (project_id, number)
);

CREATE INDEX IF NOT EXISTS idx_github_issues_project ON github_issues(project_id, updated_at_github DESC);

-- Pull requests (one row per GitHub PR within a registered project).
CREATE TABLE IF NOT EXISTS github_pull_requests (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  github_pr_id BIGINT NOT NULL,
  number INT NOT NULL,
  state TEXT,
  title TEXT,
  body TEXT,
  author_login TEXT,
  url TEXT,
  merged BOOLEAN,
  merged_at_github TIMESTAMPTZ,
  created_at_github TIMESTAMPTZ,
  updated_at_github TIMESTAMPTZ,
  closed_at_github TIMESTAMPTZ,
  last_seen_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (project_id, github_pr_id),
  UNIQUE (project_id, number)
);

CREATE INDEX IF NOT EXISTS idx_github_prs_project ON github_pull_requests(project_id, updated_at_github DESC);

-- DB-backed job queue for "heavy" sync work (later swappable to NATS).
CREATE TABLE IF NOT EXISTS sync_jobs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  project_id UUID NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  job_type TEXT NOT NULL CHECK (job_type IN ('sync_issues', 'sync_prs')),
  status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN ('pending', 'running', 'completed', 'failed')),
  run_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  attempts INT NOT NULL DEFAULT 0,
  last_error TEXT,
  locked_at TIMESTAMPTZ,
  locked_by TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_sync_jobs_pending ON sync_jobs(status, run_at);
CREATE INDEX IF NOT EXISTS idx_sync_jobs_project ON sync_jobs(project_id, created_at DESC);





















