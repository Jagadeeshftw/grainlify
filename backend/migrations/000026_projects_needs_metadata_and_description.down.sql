DROP INDEX IF EXISTS idx_projects_needs_metadata;
ALTER TABLE projects
  DROP COLUMN IF EXISTS needs_metadata,
  DROP COLUMN IF EXISTS description;
