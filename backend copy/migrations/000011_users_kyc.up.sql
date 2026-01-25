-- Add KYC verification fields to users table
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS kyc_status TEXT CHECK (kyc_status IN ('pending', 'verified', 'rejected', 'expired')) DEFAULT NULL,
  ADD COLUMN IF NOT EXISTS kyc_session_id TEXT UNIQUE,
  ADD COLUMN IF NOT EXISTS kyc_verified_at TIMESTAMPTZ,
  ADD COLUMN IF NOT EXISTS kyc_data JSONB DEFAULT '{}'::jsonb;

CREATE INDEX IF NOT EXISTS idx_users_kyc_status ON users(kyc_status) WHERE kyc_status IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_users_kyc_session_id ON users(kyc_session_id) WHERE kyc_session_id IS NOT NULL;


















