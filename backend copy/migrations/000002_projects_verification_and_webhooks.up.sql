ALTER TABLE projects
  ADD COLUMN IF NOT EXISTS github_repo_id BIGINT,
  ADD COLUMN IF NOT EXISTS verified_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS verification_error TEXT,
  ADD COLUMN IF NOT EXISTS webhook_url TEXT,
  ADD COLUMN IF NOT EXISTS webhook_created_at TIMESTAMPTZ;

CREATE TABLE IF NOT EXISTS github_webhook_deliveries (
  delivery_id TEXT PRIMARY KEY,
  event TEXT,
  received_at TIMESTAMPTZ NOT NULL DEFAULT now()
);









