CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  role TEXT NOT NULL DEFAULT 'contributor' CHECK (role IN ('contributor', 'maintainer', 'admin')),
  display_name TEXT,
  github_user_id BIGINT UNIQUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS wallets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  wallet_type TEXT NOT NULL CHECK (wallet_type IN ('evm', 'stellar_ed25519', 'stellar_secp256k1')),
  address TEXT NOT NULL,
  public_key TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  UNIQUE (wallet_type, address)
);

CREATE INDEX IF NOT EXISTS idx_wallets_user_id ON wallets(user_id);

CREATE TABLE IF NOT EXISTS auth_nonces (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  wallet_type TEXT NOT NULL CHECK (wallet_type IN ('evm', 'stellar_ed25519', 'stellar_secp256k1')),
  address TEXT NOT NULL,
  nonce TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL,
  used_at TIMESTAMPTZ,
  UNIQUE (wallet_type, address, nonce)
);

CREATE INDEX IF NOT EXISTS idx_auth_nonces_lookup ON auth_nonces(wallet_type, address, expires_at);

CREATE TABLE IF NOT EXISTS oauth_states (
  state TEXT PRIMARY KEY,
  user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK (kind IN ('github_link')),
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_oauth_states_expires ON oauth_states(expires_at);

CREATE TABLE IF NOT EXISTS github_accounts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,
  github_user_id BIGINT NOT NULL UNIQUE,
  login TEXT NOT NULL,
  access_token BYTEA NOT NULL,
  token_type TEXT,
  scope TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS projects (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  owner_user_id UUID NOT NULL REFERENCES users(id) ON DELETE RESTRICT,
  github_full_name TEXT NOT NULL UNIQUE,
  chain TEXT NOT NULL DEFAULT 'none',
  status TEXT NOT NULL DEFAULT 'pending_verification' CHECK (status IN ('pending_verification', 'verified', 'rejected')),
  webhook_id BIGINT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_projects_owner ON projects(owner_user_id);




















