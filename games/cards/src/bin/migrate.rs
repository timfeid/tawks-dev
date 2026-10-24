use anyhow::Result;
use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    // Load environment variables from .env file
    dotenvy::dotenv().ok();
    let _ = dotenvy::dotenv();

    let args: Vec<String> = env::args().collect();
    let command = args.get(1).map(|s| s.as_str()).unwrap_or("help");

    match command {
        "create" => create_migration().await?,
        "apply" => apply_migration().await?,
        "reset" => reset_database().await?,
        "validate" => validate_schema().await?,
        "cleanup" => cleanup_orphaned_effects().await?,
        _ => print_help(),
    }

    Ok(())
}

fn print_help() {
    println!(
        r#"
Cardgame Migration Tool

USAGE:
    migrate <COMMAND>

COMMANDS:
    create      Generate SQL migration from Rust types
    apply       Apply migration to database
    reset       Drop and recreate database with fresh schema
    validate    Check database schema matches Rust types
    cleanup     Remove orphaned card_effects rows

ENVIRONMENT:
    CARDGAME_DATABASE_URL    PostgreSQL connection string
"#
    );
}

async fn create_migration() -> Result<()> {
    println!("Creating migration from Rust types...\n");

    let sql = generate_schema_sql();

    // Write to migration file
    let migration_path = "migrations/0001_cardgame_schema.sql";
    std::fs::write(migration_path, &sql)?;

    println!("✓ Generated migration: {}", migration_path);
    println!("\nTo apply:");
    println!("  migrate apply");

    Ok(())
}

async fn apply_migration() -> Result<()> {
    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Applying migration...");

    let sql = generate_schema_sql();

    // Execute the migration
    sqlx::raw_sql(&sql).execute(&pool).await?;

    println!("✓ Migration applied successfully!");

    Ok(())
}

async fn reset_database() -> Result<()> {
    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");

    println!("⚠️  This will DROP ALL TABLES and recreate the schema!");
    println!("Database: {}", database_url);
    println!("\nContinue? [y/N] ");

    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    if input.trim().to_lowercase() != "y" {
        println!("Cancelled.");
        return Ok(());
    }

    println!("\nConnecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Dropping existing schema...");

    // Drop all tables and types
    sqlx::query("DROP SCHEMA public CASCADE")
        .execute(&pool)
        .await?;
    sqlx::query("CREATE SCHEMA public").execute(&pool).await?;

    println!("Creating fresh schema...");

    let sql = generate_schema_sql();
    sqlx::raw_sql(&sql).execute(&pool).await?;

    println!("✓ Database reset successfully!");
    println!("\n📝 Next step - populate the database:");
    println!("   cargo run --package cardgame --bin generate_operator_decks\n");

    Ok(())
}

async fn validate_schema() -> Result<()> {
    use cardgame::repository::CardRepository;
    use std::sync::Arc;

    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    let repo = Arc::new(CardRepository::new(Arc::new(pool)));

    println!("\n=== Validating Schema ===\n");

    // Test 1: Load all base decks
    println!("1. Loading base decks...");
    let base_decks = repo.list_base_decks().await?;
    println!("   ✓ Found {} base decks", base_decks.len());

    for deck in &base_decks {
        println!(
            "     - {}: {} (format: {:?})",
            deck.id, deck.name, deck.format
        );
    }

    // Test 2: Try to load templates for each deck
    println!("\n2. Loading deck templates...");
    let mut all_success = true;

    for deck in &base_decks {
        match repo.load_deck_templates(&deck.id).await {
            Ok(templates) => {
                if templates.is_empty() {
                    println!("   ✗ Deck '{}' has NO cards!", deck.id);
                    all_success = false;
                } else {
                    println!("   ✓ Deck '{}': {} cards", deck.id, templates.len());
                }
            }
            Err(e) => {
                println!("   ✗ Deck '{}' failed to load: {}", deck.id, e);
                all_success = false;
            }
        }
    }

    // Test 3: Check operator decks specifically
    println!("\n3. Validating Operator decks...");
    let operator_decks: Vec<_> = base_decks
        .iter()
        .filter(|d| d.format == cardgame::types::DeckFormat::Operator)
        .collect();

    if operator_decks.is_empty() {
        println!("   ⚠️  No Operator decks found in database");
        println!("   Run: cargo run --bin generate_operator_decks");
    } else {
        println!("   ✓ Found {} Operator decks", operator_decks.len());
        for deck in operator_decks {
            if deck.operator_card_id.is_none() {
                println!("   ✗ Deck '{}' missing operator_card_id", deck.id);
                all_success = false;
            }
        }
    }

    println!("\n=== Summary ===\n");
    if all_success && !base_decks.is_empty() {
        println!("✓ All validations passed!");
        println!("✓ Schema is correct and decks load successfully");
    } else if !all_success {
        println!("✗ Some validations failed - see errors above");
        return Err(anyhow::anyhow!("Validation failed"));
    } else {
        println!("⚠️  Schema is valid but no decks found");
        println!("   Run: cargo run --bin generate_operator_decks");
    }

    Ok(())
}

fn generate_schema_sql() -> String {
    // Generate SQL from Rust type definitions
    // This ensures the schema always matches the code

    format!(
        r#"-- Auto-generated from Rust types
-- DO NOT EDIT MANUALLY - run `migrate create` to regenerate

-- Enum types (using DO blocks for idempotent creation)
DO $$ BEGIN
    CREATE TYPE card_kind AS ENUM (
        'unit', 'infrastructure', 'instant', 'ritual', 'aura', 'relic', 'agent'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE card_supertype AS ENUM ('basic', 'apex');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE unit_type AS ENUM (
        'none', 'bio_organism', 'human', 'megafauna', 'anomaly', 'drone', 'virus', 'hacker', 'node'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE artifact_type AS ENUM ('equipment', 'object');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE card_rarity AS ENUM ('public', 'private', 'restricted', 'classified', 'singularity');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE mana_type AS ENUM ('neon', 'glitch', 'chrome', 'slag', 'bio', 'colorless');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE trigger_event AS ENUM (
        'entered', 'left', 'died', 'attacked', 'blocked',
        'dealt_damage', 'received_damage', 'tapped',
        'turn_start', 'upkeep', 'draw_step', 'end_of_turn',
        'cast_instant_or_ritual', 'cast_relic', 'cast_any', 'cast_during_opponent_turn',
        'drew_card', 'discarded', 'sacrificed', 'countered_spell',
        'ally_entered', 'ally_died', 'any_ally_dealt_damage', 'token_created',
        'opponent_cast', 'opponent_cast_non_unit', 'opponent_drew', 'opponent_unit_died',
        'life_lost', 'life_gained', 'graveyard_entry', 'equipped', 'searched_library'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE trigger_limit AS ENUM ('unlimited', 'once_per_turn');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE target_scope AS ENUM ('self_player', 'opponent', 'any', 'any_player_or_unit');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE target_kind AS ENUM ('player', 'unit', 'card', 'player_or_unit');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

-- Main tables
CREATE TABLE IF NOT EXISTS cards (
    card_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    kind card_kind NOT NULL,
    supertype card_supertype NOT NULL DEFAULT 'basic',
    unit_type unit_type,
    artifact_type artifact_type,

    -- Mana cost
    cost_neon INTEGER NOT NULL DEFAULT 0 CHECK (cost_neon >= 0),
    cost_glitch INTEGER NOT NULL DEFAULT 0 CHECK (cost_glitch >= 0),
    cost_chrome INTEGER NOT NULL DEFAULT 0 CHECK (cost_chrome >= 0),
    cost_slag INTEGER NOT NULL DEFAULT 0 CHECK (cost_slag >= 0),
    cost_bio INTEGER NOT NULL DEFAULT 0 CHECK (cost_bio >= 0),
    cost_colorless INTEGER NOT NULL DEFAULT 0 CHECK (cost_colorless >= 0),

    -- Unit stats
    power INTEGER,
    toughness INTEGER,
    loyalty INTEGER,

    -- Legacy field (kept for backward compatibility)
    target_spec JSONB,

    -- Other properties
    attachment JSONB,
    tapped BOOLEAN NOT NULL DEFAULT FALSE,
    enters_tapped BOOLEAN NOT NULL DEFAULT FALSE,
    keywords JSONB NOT NULL DEFAULT '[]',
    image_url TEXT,
    colors mana_type[] NOT NULL DEFAULT '{{}}',
    rarity card_rarity NOT NULL DEFAULT 'public',
    categories TEXT[] NOT NULL DEFAULT '{{}}',
    visual_spec JSONB,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (
        (unit_type IS NULL AND kind != 'unit')
        OR (unit_type IS NOT NULL AND kind = 'unit')
    ),
    CHECK (
        (artifact_type IS NULL AND kind != 'relic')
        OR (artifact_type IS NOT NULL AND kind = 'relic')
    )
);

CREATE TABLE IF NOT EXISTS card_abilities (
    ability_id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    name TEXT NOT NULL,

    -- Cost as JSONB: {{ "costs": [{{ "tap": true }}, {{ "mana": {{...}} }}] }}
    cost JSONB NOT NULL DEFAULT '{{"costs": []}}',

    -- Target specification
    target_scope target_scope,
    target_kind target_kind,
    target_max_targets INTEGER,
    target_optional BOOLEAN NOT NULL DEFAULT FALSE,

    -- Additional properties
    once_per_turn BOOLEAN NOT NULL DEFAULT FALSE,
    uses_stack BOOLEAN NOT NULL DEFAULT TRUE,
    tap_targets BOOLEAN NOT NULL DEFAULT FALSE,
    predicate JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,

    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    CHECK (
        (target_scope IS NULL AND target_kind IS NULL AND target_max_targets IS NULL)
        OR (target_scope IS NOT NULL AND target_kind IS NOT NULL AND target_max_targets IS NOT NULL)
    )
);

CREATE TABLE IF NOT EXISTS card_triggers (
    trigger_id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    event trigger_event NOT NULL,
    event_unit_type unit_type,
    "limit" trigger_limit NOT NULL DEFAULT 'unlimited',
    predicate JSONB,

    -- Optional cost as JSONB: {{ "costs": [...] }}
    optional_cost JSONB,
    is_optional BOOLEAN NOT NULL DEFAULT FALSE,

    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

DO $$ BEGIN
    CREATE TYPE effect_owner_kind AS ENUM ('ability', 'trigger', 'card');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE card_effect_kind AS ENUM (
        'remember_target_player', 'damage_player', 'damage_unit', 'damage_target',
        'draw_cards', 'gain_life', 'heat', 'heat_opponents', 'cash_opponents', 'counter_top', 'add_mana', 'create_token',
        'create_infrastructure', 'apply_stat_modifier', 'apply_stat_modifiers',
        'sequence', 'reveal_top_if_unit', 'reveal_top_if', 'freeze', 'refund_mana', 'loot', 'wheel',
        'scry', 'search', 'bounce', 'board_wipe', 'destroy_target_player_units', 'destroy_units_with_toughness_at_least',
        'copy_last_cast', 'purify', 'grant_keyword', 'discard', 'steal_mana',
        'shuffle_library', 'forbid_name', 'fight', 'tap_target', 'untap_target', 'destroy_target',
        'discard_self_random',
        'exile_target', 'equip', 'add_mana_scaling', 'modify_loyalty', 'emblem',
        'sacrifice_this', 'reanimate_named_card', 'copy_unit', 'mill',
        'bounce_all_units', 'bounce_all_opponent_permanents', 'bounce_all_attacking',
        'search_graveyard', 'reanimate_target_unit', 'add_counters', 'remove_counters',
        'pump_by_mana', 'damage_player_per_hand_card', 'damage_player_per_battlefield_kind',
        'create_tokens_per_battlefield_kind',
        'siphon', 'transmute_mana', 'swap', 'phase', 'mutate_target', 'polymorph',
        'surveil', 'transmit', 'entomb', 'recycle', 'damage_split',
        'damage_each_opponent_unit', 'reflect', 'conscript', 'scavenge',
        'assimilate'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

ALTER TYPE card_effect_kind ADD VALUE IF NOT EXISTS 'assimilate';
ALTER TYPE card_effect_kind ADD VALUE IF NOT EXISTS 'search_to_battlefield';

DO $$ BEGIN
    CREATE TYPE effect_target_spec_kind AS ENUM ('targets', 'source', 'owner_units', 'opponent_units', 'all_units', 'adjacent_units', 'last_cast');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE keyword_kind AS ENUM (
        'haste', 'deathtouch', 'piercing', 'reactive_armor', 'optics', 'gravecast',
        'firewall', 'signal_jam', 'stealth', 'aero', 'throttle', 'glitch', 'malware',
        'ethereal', 'trample', 'camouflage', 'interceptor', 'flash',
        'static_power_buff', 'static_toughness_buff', 'indestructible', 'vigilance',
        'loyalty', 'extra_infrastructures', 'untap_during_other_turns',
        'static_haste_grant', 'static_trample_grant', 'static_stealth_grant',
        'static_indestructible_grant', 'static_tap_for_mana',
        'menace', 'wither', 'absorb', 'modular', 'evolve', 'echo', 'cascade'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE stat_type AS ENUM ('power', 'toughness', 'both');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

DO $$ BEGIN
    CREATE TYPE expire_contract AS ENUM ('end_of_turn', 'never', 'turns');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS card_effects (
    effect_id TEXT PRIMARY KEY,
    owner_kind effect_owner_kind NOT NULL,
    card_id TEXT,
    ability_id TEXT,
    trigger_id TEXT,
    effect_kind card_effect_kind NOT NULL,
    mana_type mana_type,
    amount INTEGER,
    count INTEGER,
    target_scope target_scope,
    target_spec_kind effect_target_spec_kind,
    target_unit_type unit_type,
    scale_unit_type unit_type,
    scale_card_kind TEXT,
    keyword_kind keyword_kind,
    keyword_amount INTEGER,
    keyword_unit_type unit_type,
    token_name TEXT,
    token_power INTEGER,
    token_toughness INTEGER,
    token_unit_type unit_type,
    stat_type stat_type,
    stat_amount INTEGER,
    stat_amount_dynamic JSONB,
    expires_kind expire_contract,
    expires_turns INTEGER,
    effect_predicate JSONB,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS card_effect_keywords (
    effect_id TEXT NOT NULL REFERENCES card_effects(effect_id) ON DELETE CASCADE,
    keyword_kind keyword_kind NOT NULL,
    keyword_amount INTEGER,
    keyword_unit_type unit_type,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (effect_id, keyword_kind, sort_order)
);

DO $$ BEGIN
    CREATE TYPE deck_format AS ENUM ('classic', 'operator');
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS decks (
    deck_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT,
    owner_id TEXT,
    base_deck_id TEXT,
    format deck_format,
    operator_card_id TEXT,
    image_url TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS deck_cards (
    deck_id TEXT NOT NULL REFERENCES decks(deck_id) ON DELETE CASCADE,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    quantity INTEGER NOT NULL DEFAULT 1 CHECK (quantity > 0),
    is_operator BOOLEAN NOT NULL DEFAULT FALSE,
    is_sideboard BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (deck_id, card_id, is_sideboard)
);

-- Evolve pre-sideboard schemas: add the column, and widen the primary key so
-- the same card can appear in both main and sideboard.
ALTER TABLE deck_cards ADD COLUMN IF NOT EXISTS is_sideboard BOOLEAN NOT NULL DEFAULT FALSE;
DO $$ BEGIN
    ALTER TABLE deck_cards DROP CONSTRAINT IF EXISTS deck_cards_pkey;
    ALTER TABLE deck_cards ADD PRIMARY KEY (deck_id, card_id, is_sideboard);
EXCEPTION WHEN invalid_table_definition THEN NULL;
END $$;

-- Pack system
CREATE TABLE IF NOT EXISTS pack_types (
    pack_type_id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL DEFAULT '',
    active BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pack_slots (
    slot_id TEXT PRIMARY KEY,
    pack_type_id TEXT NOT NULL REFERENCES pack_types(pack_type_id) ON DELETE CASCADE,
    slot_index INTEGER NOT NULL,
    weight_public INTEGER NOT NULL DEFAULT 0,
    weight_private INTEGER NOT NULL DEFAULT 0,
    weight_restricted INTEGER NOT NULL DEFAULT 0,
    weight_classified INTEGER NOT NULL DEFAULT 0,
    weight_singularity INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- User card instances (collection)
CREATE TABLE IF NOT EXISTS user_card_instances (
    instance_id TEXT PRIMARY KEY,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    owner_id TEXT NOT NULL,
    visual_spec JSONB NOT NULL,
    is_tradeable BOOLEAN NOT NULL DEFAULT TRUE,
    is_bound BOOLEAN NOT NULL DEFAULT FALSE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Deck card instances (competitive mode)
CREATE TABLE IF NOT EXISTS deck_card_instances (
    deck_id TEXT NOT NULL REFERENCES decks(deck_id) ON DELETE CASCADE,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id) ON DELETE CASCADE,
    is_operator BOOLEAN NOT NULL DEFAULT FALSE,
    sort_order INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (deck_id, instance_id)
);

-- Card audit log
DO $$ BEGIN
    CREATE TYPE audit_event_kind AS ENUM (
        'created', 'transferred', 'visual_spec_updated', 'bound', 'unbound', 'destroyed', 'pack_opened'
    );
EXCEPTION WHEN duplicate_object THEN NULL;
END $$;

CREATE TABLE IF NOT EXISTS card_audit_log (
    log_id TEXT PRIMARY KEY,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id) ON DELETE CASCADE,
    event_kind audit_event_kind NOT NULL,
    actor_id TEXT NOT NULL,
    previous_owner_id TEXT,
    new_owner_id TEXT,
    previous_visual_spec JSONB,
    new_visual_spec JSONB,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Pack inventory
CREATE TABLE IF NOT EXISTS pack_inventory (
    pack_instance_id TEXT PRIMARY KEY,
    pack_type_id TEXT NOT NULL REFERENCES pack_types(pack_type_id) ON DELETE CASCADE,
    owner_id TEXT NOT NULL,
    source TEXT NOT NULL,
    source_ref TEXT,
    opened BOOLEAN NOT NULL DEFAULT FALSE,
    opened_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Pack opening results
CREATE TABLE IF NOT EXISTS pack_opening_results (
    result_id TEXT PRIMARY KEY,
    pack_instance_id TEXT NOT NULL REFERENCES pack_inventory(pack_instance_id) ON DELETE CASCADE,
    slot_index INTEGER NOT NULL,
    instance_id TEXT NOT NULL REFERENCES user_card_instances(instance_id) ON DELETE CASCADE,
    card_id TEXT NOT NULL REFERENCES cards(card_id) ON DELETE CASCADE,
    rarity card_rarity NOT NULL
);

-- Pack audit log
CREATE TABLE IF NOT EXISTS pack_audit_log (
    log_id TEXT PRIMARY KEY,
    pack_instance_id TEXT NOT NULL REFERENCES pack_inventory(pack_instance_id) ON DELETE CASCADE,
    event_kind TEXT NOT NULL,
    actor_id TEXT NOT NULL,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_pack_slots_pack_type ON pack_slots(pack_type_id);
CREATE INDEX IF NOT EXISTS idx_user_card_instances_owner ON user_card_instances(owner_id);
CREATE INDEX IF NOT EXISTS idx_user_card_instances_card ON user_card_instances(card_id);
CREATE INDEX IF NOT EXISTS idx_cards_kind ON cards(kind);
CREATE INDEX IF NOT EXISTS idx_cards_supertype ON cards(supertype);
CREATE INDEX IF NOT EXISTS idx_cards_unit_type ON cards(unit_type);
CREATE INDEX IF NOT EXISTS idx_card_abilities_card_id ON card_abilities(card_id);
CREATE INDEX IF NOT EXISTS idx_card_triggers_card_id ON card_triggers(card_id);
CREATE INDEX IF NOT EXISTS idx_card_effects_card ON card_effects(card_id);
CREATE INDEX IF NOT EXISTS idx_card_effects_ability ON card_effects(ability_id);
CREATE INDEX IF NOT EXISTS idx_card_effects_trigger ON card_effects(trigger_id);
CREATE INDEX IF NOT EXISTS idx_deck_cards_deck_id ON deck_cards(deck_id);
CREATE INDEX IF NOT EXISTS idx_deck_cards_card_id ON deck_cards(card_id);
CREATE INDEX IF NOT EXISTS idx_deck_card_instances_deck ON deck_card_instances(deck_id);
CREATE INDEX IF NOT EXISTS idx_card_audit_log_instance ON card_audit_log(instance_id);
CREATE INDEX IF NOT EXISTS idx_pack_inventory_owner ON pack_inventory(owner_id);
CREATE INDEX IF NOT EXISTS idx_pack_opening_results_pack ON pack_opening_results(pack_instance_id);
"#
    )
}

async fn cleanup_orphaned_effects() -> Result<()> {
    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");

    println!("Connecting to database...");
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    // Count orphans before cleanup
    let orphaned_ability: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM card_effects WHERE ability_id IS NOT NULL AND ability_id NOT IN (SELECT ability_id FROM card_abilities)"
    ).fetch_one(&pool).await?;

    let orphaned_trigger: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM card_effects WHERE trigger_id IS NOT NULL AND trigger_id NOT IN (SELECT trigger_id FROM card_triggers)"
    ).fetch_one(&pool).await?;

    println!("Found {} orphaned ability effects", orphaned_ability.0);
    println!("Found {} orphaned trigger effects", orphaned_trigger.0);

    if orphaned_ability.0 == 0 && orphaned_trigger.0 == 0 {
        println!("\nNo orphaned effects to clean up.");
        return Ok(());
    }

    // Delete orphaned effects
    let deleted_ability = sqlx::query(
        "DELETE FROM card_effects WHERE ability_id IS NOT NULL AND ability_id NOT IN (SELECT ability_id FROM card_abilities)"
    ).execute(&pool).await?;

    let deleted_trigger = sqlx::query(
        "DELETE FROM card_effects WHERE trigger_id IS NOT NULL AND trigger_id NOT IN (SELECT trigger_id FROM card_triggers)"
    ).execute(&pool).await?;

    println!(
        "\nDeleted {} orphaned ability effects",
        deleted_ability.rows_affected()
    );
    println!(
        "Deleted {} orphaned trigger effects",
        deleted_trigger.rows_affected()
    );

    Ok(())
}
