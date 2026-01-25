-- Revert: Remove 'not_started' from allowed kyc_status values
ALTER TABLE users
  DROP CONSTRAINT IF EXISTS users_kyc_status_check;

ALTER TABLE users
  ADD CONSTRAINT users_kyc_status_check CHECK (kyc_status IN ('pending', 'verified', 'rejected', 'expired') OR kyc_status IS NULL);


















