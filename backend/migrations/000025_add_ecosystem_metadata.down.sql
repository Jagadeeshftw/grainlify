-- Remove metadata fields from ecosystems table
ALTER TABLE ecosystems
DROP COLUMN IF EXISTS short_description,
DROP COLUMN IF EXISTS languages,
DROP COLUMN IF EXISTS key_areas,
DROP COLUMN IF EXISTS technologies;
