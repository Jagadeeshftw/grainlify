-- Add redirect_uri column to oauth_states table
-- This allows storing the frontend URL to redirect to after OAuth callback
-- Enables single OAuth callback URL to work with multiple frontend deployments
ALTER TABLE oauth_states
  ADD COLUMN IF NOT EXISTS redirect_uri TEXT;

