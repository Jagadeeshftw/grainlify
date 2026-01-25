-- Add stars_count and forks_count to projects table
ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS stars_count INTEGER DEFAULT 0,
  ADD COLUMN IF NOT EXISTS forks_count INTEGER DEFAULT 0;

-- Create indexes for sorting/filtering
CREATE INDEX IF NOT EXISTS idx_projects_stars_count ON projects(stars_count) WHERE stars_count > 0;
CREATE INDEX IF NOT EXISTS idx_projects_forks_count ON projects(forks_count) WHERE forks_count > 0;

