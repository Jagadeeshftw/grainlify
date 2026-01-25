-- Add social media links to users table
ALTER TABLE users
  ADD COLUMN IF NOT EXISTS telegram TEXT,
  ADD COLUMN IF NOT EXISTS linkedin TEXT,
  ADD COLUMN IF NOT EXISTS whatsapp TEXT,
  ADD COLUMN IF NOT EXISTS twitter TEXT,
  ADD COLUMN IF NOT EXISTS discord TEXT;

