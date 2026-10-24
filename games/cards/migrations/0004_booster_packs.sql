-- Booster pack system: types, slots, inventory, and opening results

CREATE TABLE IF NOT EXISTS pack_types (
    pack_type_id TEXT NOT NULL PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE TABLE IF NOT EXISTS pack_slots (
    slot_id TEXT NOT NULL PRIMARY KEY,
    pack_type_id TEXT NOT NULL REFERENCES pack_types(pack_type_id),
    slot_index INTEGER NOT NULL,
    weight_public INTEGER NOT NULL DEFAULT 0,
    weight_private INTEGER NOT NULL DEFAULT 0,
    weight_restricted INTEGER NOT NULL DEFAULT 0,
    weight_classified INTEGER NOT NULL DEFAULT 0,
    weight_singularity INTEGER NOT NULL DEFAULT 0,
    UNIQUE(pack_type_id, slot_index)
);

CREATE TABLE IF NOT EXISTS pack_inventory (
    pack_instance_id TEXT NOT NULL PRIMARY KEY,
    pack_type_id TEXT NOT NULL REFERENCES pack_types(pack_type_id),
    owner_id TEXT NOT NULL,
    source TEXT NOT NULL DEFAULT 'store',
    source_ref TEXT,
    opened BOOLEAN NOT NULL DEFAULT FALSE,
    opened_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_pack_inventory_owner ON pack_inventory (owner_id, created_at DESC);

CREATE TABLE IF NOT EXISTS pack_opening_results (
    result_id TEXT NOT NULL PRIMARY KEY,
    pack_instance_id TEXT NOT NULL REFERENCES pack_inventory(pack_instance_id),
    slot_index INTEGER NOT NULL,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id),
    card_id TEXT NOT NULL REFERENCES cards(card_id),
    rarity card_rarity NOT NULL,
    UNIQUE(pack_instance_id, slot_index)
);

CREATE TABLE IF NOT EXISTS pack_audit_log (
    log_id TEXT NOT NULL PRIMARY KEY,
    pack_instance_id TEXT,
    event_kind TEXT NOT NULL,
    actor_id TEXT,
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
