-- Add 'in_review' to allowed kyc_status values
-- This is needed because Didit returns "In Review" status which we map to 'in_review'
-- The status flow is: not_started -> pending -> in_review -> verified/rejected/expired

ALTER TABLE users
  DROP CONSTRAINT IF EXISTS users_kyc_status_check;

ALTER TABLE users
  ADD CONSTRAINT users_kyc_status_check CHECK (kyc_status IN ('not_started', 'pending', 'in_review', 'verified', 'rejected', 'expired') OR kyc_status IS NULL);


















