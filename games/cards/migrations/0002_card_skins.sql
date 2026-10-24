CREATE TYPE card_rarity AS ENUM (
    'public',
    'private',
    'restricted',
    'classified',
    'singularity'
);

-- Categories are just text tags for now to keep it flexible
ALTER TABLE cards ADD COLUMN rarity card_rarity NOT NULL DEFAULT 'public';
ALTER TABLE cards ADD COLUMN categories TEXT[] NOT NULL DEFAULT '{}';
ALTER TABLE cards ADD COLUMN visual_spec JSONB;
