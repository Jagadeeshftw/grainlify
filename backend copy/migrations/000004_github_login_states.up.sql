-- Allow OAuth states that are not tied to an existing user (github_login).
ALTER TABLE oauth_states
  ALTER COLUMN user_id DROP NOT NULL;

-- Expand allowed kinds.
ALTER TABLE oauth_states
  DROP CONSTRAINT IF EXISTS oauth_states_kind_check;

ALTER TABLE oauth_states
  ADD CONSTRAINT oauth_states_kind_check CHECK (kind IN ('github_link', 'github_login'));



















