ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS ecosystem_id UUID REFERENCES ecosystems(id) ON DELETE SET NULL;

CREATE INDEX IF NOT EXISTS idx_projects_ecosystem_id ON projects(ecosystem_id);



















