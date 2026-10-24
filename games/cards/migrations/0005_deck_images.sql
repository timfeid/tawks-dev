-- Add image_url to decks for deck cover art
ALTER TABLE decks ADD COLUMN IF NOT EXISTS image_url TEXT;
