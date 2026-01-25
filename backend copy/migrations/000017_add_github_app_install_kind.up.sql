-- Add 'github_app_install' to allowed oauth_states kinds
ALTER TABLE oauth_states
  DROP CONSTRAINT IF EXISTS oauth_states_kind_check;

ALTER TABLE oauth_states
  ADD CONSTRAINT oauth_states_kind_check CHECK (kind IN ('github_link', 'github_login', 'github_app_install'));















