-- Add 'not_started' to allowed kyc_status values
-- This is needed because Didit returns "Not Started" status which we map to 'not_started'
-- The status flow is: not_started -> pending -> verified/rejected/expired

ALTER TABLE users
  DROP CONSTRAINT IF EXISTS users_kyc_status_check;

ALTER TABLE users
  ADD CONSTRAINT users_kyc_status_check CHECK (kyc_status IN ('not_started', 'pending', 'verified', 'rejected', 'expired') OR kyc_status IS NULL);


















