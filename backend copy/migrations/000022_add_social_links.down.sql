-- Remove social media links from users table
ALTER TABLE users
  DROP COLUMN IF EXISTS telegram,
  DROP COLUMN IF EXISTS linkedin,
  DROP COLUMN IF EXISTS whatsapp,
  DROP COLUMN IF EXISTS twitter,
  DROP COLUMN IF EXISTS discord;

