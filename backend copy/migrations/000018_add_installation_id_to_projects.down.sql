-- Remove installation_id and deleted_at columns
ALTER TABLE projects
  DROP COLUMN IF EXISTS github_app_installation_id;
  
ALTER TABLE projects
  DROP COLUMN IF EXISTS deleted_at;















