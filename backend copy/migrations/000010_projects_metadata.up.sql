-- Add language, tags, and category fields to projects table
ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS language TEXT,
  ADD COLUMN IF NOT EXISTS tags JSONB DEFAULT '[]'::jsonb,
  ADD COLUMN IF NOT EXISTS category TEXT;

-- Create indexes for filtering
CREATE INDEX IF NOT EXISTS idx_projects_language ON projects(language) WHERE language IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_projects_category ON projects(category) WHERE category IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_projects_tags ON projects USING GIN(tags) WHERE tags IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_projects_status ON projects(status) WHERE status = 'verified';


















