-- Mark existing projects as needing metadata if they lack description or ecosystem,
-- so "Complete setup" appears in the Maintainers repo dropdown and they don't appear on Browse until completed.
UPDATE projects
SET needs_metadata = true
WHERE needs_metadata = false
  AND deleted_at IS NULL
  AND (
    (description IS NULL OR TRIM(description) = '')
    OR ecosystem_id IS NULL
  );
