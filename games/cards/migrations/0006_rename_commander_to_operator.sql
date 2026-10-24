-- Rename commander -> operator, planeswalker -> agent, legendary -> apex

-- 1. Rename columns: commander_card_id -> operator_card_id
ALTER TABLE decks RENAME COLUMN commander_card_id TO operator_card_id;

-- 2. Rename columns: is_commander -> is_operator
ALTER TABLE deck_cards RENAME COLUMN is_commander TO is_operator;
ALTER TABLE deck_card_instances RENAME COLUMN is_commander TO is_operator;

-- 3. card_kind enum: rename 'planeswalker' -> 'agent'
ALTER TYPE card_kind ADD VALUE IF NOT EXISTS 'agent';
UPDATE cards SET kind = 'agent' WHERE kind = 'planeswalker';

-- 4. card_supertype enum: rename 'legendary' -> 'apex'
ALTER TYPE card_supertype ADD VALUE IF NOT EXISTS 'apex';
UPDATE cards SET supertype = 'apex' WHERE supertype = 'legendary';

-- 5. deck_format enum: rename 'commander' -> 'operator'
ALTER TYPE deck_format ADD VALUE IF NOT EXISTS 'operator';
UPDATE decks SET format = 'operator' WHERE format = 'commander';
