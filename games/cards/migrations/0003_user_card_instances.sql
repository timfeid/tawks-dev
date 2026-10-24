-- User-owned card instances with audit logging
-- Introduces: game_mode enum, user_card_instances, card_audit_log, deck_card_instances
-- Removes: visual_spec from cards template table

-- Game mode enum (orthogonal to deck_format)
CREATE TYPE game_mode AS ENUM ('competitive', 'casual');

-- Individual card copies owned by users
CREATE TABLE IF NOT EXISTS user_card_instances (
    instance_id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    owner_id TEXT NOT NULL,
    visual_spec JSONB NOT NULL DEFAULT '{}',
    is_tradeable BOOLEAN NOT NULL DEFAULT TRUE,
    is_bound BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_user_card_instances_owner ON user_card_instances(owner_id);
CREATE INDEX IF NOT EXISTS idx_user_card_instances_owner_card ON user_card_instances(owner_id, card_id);

-- Append-only audit log for card instance lifecycle events
CREATE TYPE audit_event_kind AS ENUM (
    'created',
    'transferred',
    'visual_spec_updated',
    'bound',
    'unbound',
    'destroyed'
);

CREATE TABLE IF NOT EXISTS card_audit_log (
    log_id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id),
    event_kind audit_event_kind NOT NULL,
    actor_id TEXT NOT NULL,
    previous_owner_id TEXT,
    new_owner_id TEXT,
    previous_visual_spec JSONB,
    new_visual_spec JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_card_audit_log_instance ON card_audit_log(instance_id);
CREATE INDEX IF NOT EXISTS idx_card_audit_log_created ON card_audit_log(created_at);

-- Links decks to specific owned card instances (one row per card slot)
CREATE TABLE IF NOT EXISTS deck_card_instances (
    deck_id TEXT NOT NULL REFERENCES decks(deck_id) ON DELETE CASCADE,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id),
    is_commander BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (deck_id, instance_id)
);

CREATE INDEX IF NOT EXISTS idx_deck_card_instances_deck ON deck_card_instances(deck_id);

-- Remove visual_spec from card templates (now lives on user_card_instances)
ALTER TABLE cards DROP COLUMN IF EXISTS visual_spec;
