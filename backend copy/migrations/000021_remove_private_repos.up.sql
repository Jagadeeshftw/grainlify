-- Note: This migration is a no-op placeholder
-- The actual removal of private repos happens in the application layer
-- when fetching projects via /projects/mine endpoint, as we need GitHub API access
-- to determine if a repo is private. Private repos are automatically soft-deleted
-- (deleted_at set) when they are detected during project fetching.
--
-- If you need to manually clean up private repos, you can run:
-- UPDATE projects SET deleted_at = now() WHERE <conditions>;
-- But this requires external knowledge of which repos are private.

