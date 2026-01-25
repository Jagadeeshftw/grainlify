-- Add installation_id to track GitHub App installations
ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS github_app_installation_id TEXT;

-- Create index for efficient lookups by installation_id
CREATE INDEX IF NOT EXISTS idx_projects_installation_id ON projects(github_app_installation_id) WHERE github_app_installation_id IS NOT NULL;

-- Add deleted_at for soft deletes
ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS deleted_at TIMESTAMPTZ;

-- Create index for filtering active projects
CREATE INDEX IF NOT EXISTS idx_projects_deleted_at ON projects(deleted_at) WHERE deleted_at IS NULL;















