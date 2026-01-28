-- Add metadata fields to ecosystems table
ALTER TABLE ecosystems
ADD COLUMN short_description TEXT,
ADD COLUMN languages JSONB,
ADD COLUMN key_areas JSONB,
ADD COLUMN technologies JSONB;

-- Add comments for documentation
COMMENT ON COLUMN ecosystems.short_description IS 'Brief 1-2 sentence description for sidebar display';
COMMENT ON COLUMN ecosystems.languages IS 'Array of language objects with name and percentage, e.g. [{"name": "TypeScript", "percentage": 60}]';
COMMENT ON COLUMN ecosystems.key_areas IS 'Array of key area objects with title and description, e.g. [{"title": "DeFi", "description": "Decentralized finance"}]';
COMMENT ON COLUMN ecosystems.technologies IS 'Array of technology description strings, e.g. ["TypeScript for smart contracts"]';
