-- Remove stars_count and forks_count columns
ALTER TABLE projects
  DROP COLUMN IF EXISTS stars_count,
  DROP COLUMN IF EXISTS forks_count;

