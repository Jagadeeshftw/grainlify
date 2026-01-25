-- Revert: Remove 'in_review' from allowed kyc_status values
ALTER TABLE users
  DROP CONSTRAINT IF EXISTS users_kyc_status_check;

ALTER TABLE users
  ADD CONSTRAINT users_kyc_status_check CHECK (kyc_status IN ('not_started', 'pending', 'verified', 'rejected', 'expired') OR kyc_status IS NULL);


















