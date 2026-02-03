ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS needs_metadata BOOLEAN NOT NULL DEFAULT false,
  ADD COLUMN IF NOT EXISTS description TEXT;

CREATE INDEX IF NOT EXISTS idx_projects_needs_metadata ON projects(owner_user_id, needs_metadata) WHERE needs_metadata = true;
