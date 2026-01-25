-- Remove chain column from projects table (redundant with ecosystem)
ALTER TABLE projects
  DROP COLUMN IF EXISTS chain;


















