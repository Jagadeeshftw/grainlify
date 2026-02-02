-- Add configurable detail fields for ecosystem detail page (Overview, Links, Key Areas, Technologies)
ALTER TABLE ecosystems ADD COLUMN IF NOT EXISTS about TEXT;
ALTER TABLE ecosystems ADD COLUMN IF NOT EXISTS links JSONB DEFAULT '[]';
ALTER TABLE ecosystems ADD COLUMN IF NOT EXISTS key_areas JSONB DEFAULT '[]';
ALTER TABLE ecosystems ADD COLUMN IF NOT EXISTS technologies JSONB DEFAULT '[]';
