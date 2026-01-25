-- Remove redirect_uri column from oauth_states table
ALTER TABLE oauth_states
  DROP COLUMN IF EXISTS redirect_uri;

