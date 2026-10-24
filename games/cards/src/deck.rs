use anyhow::{Result, anyhow};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use std::collections::{HashMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};

use crate::card::*;
use crate::card_catalog;
use crate::sim::patch::BalancePatch;
use crate::types::*;

#[derive(Default, Clone)]
pub struct DeckTemplates {
    pub neon_hub: Option<Card>,
}

#[derive(Clone)]
pub struct DeckDefinition {
    pub id: &'static str,
    pub name: &'static str,
    pub description: &'static str,
}

#[derive(Clone)]
pub struct DeckTemplateEntry {
    pub card: Card,
    pub quantity: i32,
}

#[derive(Clone)]
pub struct DeckCatalog {
    pub id: String,
    pub name: String,
    pub description: String,
    pub cards: Vec<DeckTemplateEntry>,
}

pub fn deck_definitions() -> Vec<DeckDefinition> {
    let mut defs = mana_clash_deck_definitions();
    defs.extend(card_catalog::syndicate::deck_definitions());
    defs
}

fn mana_clash_deck_definitions() -> Vec<DeckDefinition> {
    vec![
        DeckDefinition {
            id: "neon",
            name: "Flicker Strike",
            description: "Speed and evasion. Flickering units, unblockable attacks, and tempo control.",
        },
        DeckDefinition {
            id: "glitch",
            name: "Mind Rot",
            description: "Chaos and disruption. Discard effects, library scrambling, and attrition.",
        },
        DeckDefinition {
            id: "chrome",
            name: "Forge Protocol",
            description: "Order and protection. Artifacts, card advantage, and taxing effects.",
        },
        DeckDefinition {
            id: "slag",
            name: "Blood Furnace",
            description: "Brute force and sacrifice. Burn spells, death triggers, and explosive damage.",
        },
        DeckDefinition {
            id: "bio",
            name: "Roots of Infinity",
            description: "Evolution and growth. Ramp, tokens, and overwhelming board presence.",
        },
        DeckDefinition {
            id: "rush",
            name: "Overclock Rush",
            description: "Slag/Neon ultra-aggro. Haste and Stealth for an 8-round clock.",
        },
        DeckDefinition {
            id: "takeover",
            name: "Viral Takeover",
            description: "Bio/Glitch infestation. Force discards, then Assimilate the best threats.",
        },
        DeckDefinition {
            id: "fortress",
            name: "Relic Fortress",
            description: "Chrome/Bio scaling. Stack relics to fuel relic-scaling drones; grow tall creatures.",
        },
    ]
}

fn clone_template(template: &Card, new_id: &str) -> Card {
    let mut card = template.clone();
    card.template_id = Some(template.id.clone());
    card.id = new_id.to_string();
    card.tapped = false;
    card.attached_to = None;
    card.remembered_player_id = None;
    card.attachment = None;
    card.summoning_sickness = false;
    card.burnout = false;
    card
}

/// After building a deck from templates, apply visual_specs from user-owned card instances.
/// Each instance is consumed once (popped from the vec) to match against game cards by template_id.
pub fn apply_instance_visual_specs(
    deck: &mut [Card],
    instances: &mut HashMap<String, Vec<crate::user_card::UserCardInstance>>,
) {
    for card in deck.iter_mut() {
        if let Some(template_id) = &card.template_id {
            if let Some(inst_list) = instances.get_mut(template_id.as_str()) {
                if let Some(instance) = inst_list.pop() {
                    card.visual_spec = Some(instance.visual_spec);
                }
            }
        }
    }
}

fn slugify_name(name: &str) -> String {
    let mut slug = String::new();
    let mut last_dash = false;
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() {
            slug.push(ch.to_ascii_lowercase());
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    slug.trim_matches('-').to_string()
}

pub fn build_deck_catalog() -> Vec<DeckCatalog> {
    let mut out = Vec::new();
    for def in deck_definitions() {
        let cards =
            build_deck_with_templates("seed", def.id, Some(1), &DeckTemplates::default(), None);
        let mut by_template: HashMap<String, DeckTemplateEntry> = HashMap::new();
        for card in cards {
            let template_id = card
                .template_id
                .clone()
                .unwrap_or_else(|| slugify_name(&card.name));
            let entry = by_template.entry(template_id.clone()).or_insert_with(|| {
                let mut template = card.clone();
                template.id = template_id.clone();
                template.tapped = false;
                template.attached_to = None;
                template.remembered_player_id = None;
                template.attachment = None;
                template.summoning_sickness = false;
                template.burnout = false;
                template.haste = template
                    .keywords
                    .iter()
                    .any(|keyword| matches!(keyword, Keyword::Haste));
                for (index, ability) in template.abilities.iter_mut().enumerate() {
                    ability.id = format!("{}::ability-{}", template.id, index + 1);
                }
                DeckTemplateEntry {
                    card: template,
                    quantity: 0,
                }
            });
            entry.quantity += 1;
        }

        let mut entries = by_template.into_values().collect::<Vec<_>>();
        entries.sort_by(|a, b| a.card.id.cmp(&b.card.id));

        out.push(DeckCatalog {
            id: def.id.to_string(),
            name: def.name.to_string(),
            description: def.description.to_string(),
            cards: entries,
        });
    }
    out
}

pub fn build_deck_from_templates(
    player_id: &str,
    seed: Option<u64>,
    templates: &[DeckTemplateEntry],
) -> Vec<Card> {
    let mut cards = Vec::new();
    let mut id_counter = 0;
    let mut next_id = |base: &str| {
        id_counter += 1;
        format!("{}-{}-{}", player_id, base, id_counter)
    };

    for entry in templates {
        let quantity = entry.quantity.max(0) as usize;
        for _ in 0..quantity {
            let id = next_id(&entry.card.id);
            cards.push(clone_template(&entry.card, &id));
        }
    }

    let mut rng = match seed {
        Some(base_seed) => {
            let mut hasher = DefaultHasher::new();
            player_id.hash(&mut hasher);
            let player_hash = hasher.finish();
            StdRng::seed_from_u64(base_seed ^ player_hash)
        }
        None => StdRng::seed_from_u64(rand::random::<u64>()),
    };
    shuffle_deck_with_rules(&mut cards, &mut rng);
    cards
}

pub fn build_operator_deck_from_templates(
    player_id: &str,
    seed: Option<u64>,
    templates: &[DeckTemplateEntry],
    operator_card_id: &str,
) -> Result<(Vec<Card>, Card)> {
    let mut cards = Vec::new();
    let mut id_counter = 0;
    let mut next_id = |base: &str| {
        id_counter += 1;
        format!("{}-{}-{}", player_id, base, id_counter)
    };

    let mut operator_template: Option<Card> = None;

    for entry in templates {
        if entry.card.id == operator_card_id {
            if entry.quantity <= 0 {
                return Err(anyhow!("Operator card has zero quantity"));
            }
            operator_template = Some(entry.card.clone());
            let quantity = (entry.quantity - 1).max(0) as usize;
            for _ in 0..quantity {
                let id = next_id(&entry.card.id);
                cards.push(clone_template(&entry.card, &id));
            }
        } else {
            let quantity = entry.quantity.max(0) as usize;
            for _ in 0..quantity {
                let id = next_id(&entry.card.id);
                cards.push(clone_template(&entry.card, &id));
            }
        }
    }

    let Some(operator_template) = operator_template else {
        return Err(anyhow!("Operator card not found in deck"));
    };

    let mut rng = match seed {
        Some(base_seed) => {
            let mut hasher = DefaultHasher::new();
            player_id.hash(&mut hasher);
            let player_hash = hasher.finish();
            StdRng::seed_from_u64(base_seed ^ player_hash)
        }
        None => StdRng::seed_from_u64(rand::random::<u64>()),
    };
    shuffle_deck_with_rules(&mut cards, &mut rng);

    let operator_id = next_id(&operator_template.id);
    let operator = clone_template(&operator_template, &operator_id);
    Ok((cards, operator))
}

/// Returns the curated card IDs for a classic deck of the given color.
/// Each entry is (card_id, quantity). Apex operator cards are excluded.
pub fn classic_deck_card_ids(deck_id: &str) -> Vec<(&'static str, i32)> {
    // Classic format: 34-card decks, max 3 copies per card.
    match deck_id {
        "neon" => vec![
            // 1-cost early defense + value (4) — cascade_protocol 3→2 (mild neon buff +0.4pp)
            ("signal_stabilizer", 2),
            ("cascade_protocol", 2),
            // 2-cost evasive threats (8)
            ("signal_blade", 2),
            ("ambush_spike", 2),
            ("flicker_drone", 2),
            ("spectral_hacker", 2),
            ("signal_intercept", 1),
            // 3-cost threats (8)
            ("phase_skipper", 2),
            ("stealth_infiltrator", 2),
            ("nullify", 1),
            ("cache_recall_drone", 2),
            // 4-cost threats (4)
            ("lord_of_the_grid", 2),
            ("master_of_signals", 2),
            // 5-cost closer (3)
            ("consecrated_server", 1),
            ("harbinger_of_static", 1),
            ("full_reboot", 2),
            // Interaction (5) — access_denied 1→2 (counter back to 2)
            ("derez", 1),
            ("access_denied", 2),
            ("recursive_spike", 1),
            ("spell_shield", 1),
            ("mass_derez", 1), // in for a 3rd stealth_infiltrator: answers wide token boards
        ],
        "glitch" => vec![
            // 1-cost disruption (5) — heavy cuts
            ("deep_inquisition", 1),
            ("static_crime", 1),
            ("entropy_spore", 1),
            ("cache_burial", 1),
            ("shrieking_static", 1),
            // 2-cost (11) — victim_of_static 2→1 (AI now redirects exile at apex,
            // overamplified glitch's removal density post-AI-changes)
            ("data_rat", 1),
            ("ravenous_worms", 1),
            ("fatal_error", 1),
            ("memory_leak", 1),
            ("system_drain", 1),
            ("logic_bomb", 1),
            ("thought_corrupt", 1),
            ("tragic_crash", 1),
            ("victim_of_static", 1),
            ("data_cascade", 1),
            // 3-cost (8) — rewrite_memory 1→2 (slow draw filler in place of cut tragic_crash)
            ("waste_not_want_not", 1),
            ("entropy_spectre", 1),
            ("null_crawler", 1),
            ("rewrite_memory", 2),
            ("bottomless_cache", 1),
            ("data_hemorrhage", 1),
            ("recursive_reclamation", 1),
            // 4-cost (11) — corrupt_deal 2→3 (extra wheel filler)
            ("hymn_to_entropy", 1),
            ("go_for_the_core", 1),
            ("system_downfall", 1),
            ("archived_alive", 1),
            ("total_corruption", 1),
            ("memetic_override", 1),
            ("data_cascade", 1),
            ("data_necromancy", 1),
            ("rise_from_the_dump", 1),
            // wrench_process in for one corrupt_deal: a cheap body against slag aggro
            ("wrench_process", 1),
            ("corrupt_deal", 2),
        ],
        "chrome" => vec![
            // 1-cost artifacts + cheap units (5) — chrome_scanner restored
            ("grim_reactor", 2),
            ("sol_core", 1),
            ("chrome_scanner", 2),
            // 2-cost bodies + value (7) — basalt_dynamo 2→1 (ramp relic trim)
            ("chrome_sentinel", 2),
            ("plating_drone", 2),
            ("basalt_dynamo", 1),
            ("exile_protocol", 2),
            ("emergency_patch", 1),
            // 3-cost interaction (6)
            ("displacement_beam", 2),
            ("drone_convergence", 2),
            ("rhystic_scanner", 2),
            // 4-cost (5)
            ("foundry_overseer", 1),
            ("fact_or_function", 2),
            ("chrome_phantom", 2),
            // Value anchors + finishers (3)
            ("chrome_bulwark", 1),
            ("steel_titan", 1),
            ("chrome_battlesphere", 1),
            // Combo + tempo (6)
            ("prototype_accelerator", 1),
            ("spellforge_engine", 1),
            ("convergence_strike", 1),
            ("data_recovery_matrix", 1),
            ("corporate_reprogramming", 1),
            ("shock_module", 1),
            // Filler (1)
            ("whir_of_chrome", 1),
        ],
        "slag" => vec![
            // 1-cost aggro openers (6) — jolt 2→3 (more efficient burn vs bio 1-toughness army)
            ("jolt", 3),
            ("slag_crawler", 2),
            ("volatile_gremlin", 1),
            // 2-cost burn (5)
            ("arc_discharge", 1),
            ("furnace_vent", 1),
            ("magma_charger", 1),
            ("molten_insight", 1),
            ("woe_stoker", 1),
            // 3-cost engines + pressure (7) — thrill_of_the_forge 1→0 (slot for jolt 2→3)
            ("ember_phoenix", 1),
            ("foundry_rabblemaster", 1),
            ("molten_phoenix", 1),
            ("yawg_thran_engineer", 1),
            ("forge_blast", 1),
            ("chain_detonation", 1),
            ("cathartic_combustion", 1),
            // 4-cost closers (4)
            ("slag_eruption", 1),
            ("forge_conversion", 1),
            ("impact_sparks", 1),
            ("furnace_seer", 1),
            // 5+ finishers (3)
            ("siege_behemoth", 2),
            ("inferno_colossus", 1),
            // Utility
            ("blood_furnace_core", 2),
            ("gremlin_propagation", 1),
            ("slag_reactor", 1),
            // Filler
            ("bonfire_of_the_smelted", 1), // restored: slag_storm hit own units too hard
            ("meltdown_protocol", 1),      // 4-mana mass -3/-3 wipe (anti-token)
            ("reforge_the_slag", 1),
            ("combustion_spirit", 1), // 4-mana 2/2 Haste + team buff/Haste
            ("forge_war_marshal", 1), // 2-mana 1/1 dies→token
        ],
        "bio" => vec![
            // 1-cost defensive wall (8)
            ("tendril_sapper", 2),
            ("mycelium_weaver", 2),
            ("bramble_seedling", 2),
            ("venomous_spore", 2),
            // 2-cost threats + utility (9) — bloom_warden 2→3 (small bio uptick, +0.4pp target)
            ("root_graft", 2),
            ("consumed_by_growth", 2),
            ("bloom_warden", 3),
            ("toxic_stalker", 1),
            ("spore_shell", 1),
            // 3-cost removal + value (7) — adaptive_polyp 1→0 (slot for bloom_warden 3rd)
            ("reclamation_tendril", 2),
            ("root_expansion", 2),
            ("regal_overgrowth", 1),
            ("grove_huntmaster", 1),
            // 4-cost threats (5)
            ("champion_of_the_thicket", 2),
            ("grove_elder", 2),
            ("predatory_strike", 1),
            // 5+ finishers (5)
            ("avenger_of_the_grove", 2), // 2nd in for perennial_witness
            ("apex_stampede", 1),
            ("thornveil_elder", 1),
            ("nest_sprouter", 1), // in for thornback_riders: a cheaper curve
            ("gaia_who_awakens", 1),
        ],
        // ── Multi-color themed decks ──────────────────────────────────────
        "rush" => vec![
            // Slag/Neon ultra-aggro (34): cheap Haste + Stealth bodies.
            ("jolt", 3),                 // 1-mana burn
            ("slag_crawler", 3),         // 1-mana slag attacker
            ("volatile_gremlin", 3),     // 1-mana 2/1 Haste + death ping
            ("neon_infiltrator", 2),     // 2/1 Haste+Stealth + draw
            ("ambush_spike", 2),         // 2/2 Flash Deathtouch
            ("ping_cutthroat", 2),       // 2/3 Stealth
            ("waveline_stalker", 2),     // 2/1 Stealth + scry
            ("cunning_netrunner", 2),    // 2/3 Stealth + ramp
            ("furnace_vent", 2),         // 2-mana burn
            ("arc_discharge", 2),        // 2-mana burn
            ("foundry_rabblemaster", 2), // 2/3 Haste + token/turn
            ("derez", 2),                // tempo bounce
            ("signal_blade", 2),         // weapon
            ("ember_phoenix", 2),        // slag recursion threat
            ("chain_detonation", 1),     // board swing
            ("harbinger_of_static", 1),  // flash bounce
            ("echo_pulse", 1),           // Gravecast draw
                                         // total: 3+3+3+2+2+2+2+2+2+2+2+2+2+2+1+1+1 = 34
        ],
        "takeover" => vec![
            // Bio/Glitch infestation: force discards, then Assimilate to grow
            // the swarm with opponent's own units.
            ("tendril_sapper", 3),
            ("adaptive_polyp", 2),
            ("mycelium_weaver", 2),
            ("spore_scout", 2),
            ("thornveil_elder", 2),
            ("bloom_warden", 2),
            ("grove_huntmaster", 2),
            ("consumed_by_growth", 2),
            ("root_graft", 2),
            ("deep_inquisition", 2),
            ("data_rat", 3),
            ("hypnotic_glitch", 2),
            ("ravenous_worms", 2),
            ("virus_disciple", 2),
            ("fatal_error", 2),
            ("memetic_override", 1),
            ("shrieking_static", 2),
        ],
        "fortress" => vec![
            // Chrome/Bio relic-scaling + growing creatures. Stack relics,
            // fuel relic-scaling drones, close with chrome finishers.
            // 1-cost (6)
            ("sol_core", 2),       // chrome 1-mana mana rock
            ("chrome_scanner", 2), // chrome relic
            ("spore_scout", 2),    // bio 1-drop
            // 2-cost bodies + relics (10)
            ("chrome_sentinel", 2),  // chrome Flash team-indestructible
            ("emergency_patch", 2),  // chrome Indestructible protect
            ("plating_drone", 2),    // Alloy Construct (+1/+1 per relic)
            ("spellskite_drone", 2), // Relic Sentinel (+2/+0 per relic)
            ("power_cell", 1),       // chrome relic
            ("symbiotic_shield", 1), // Gene Bulwark (bio anthem ritual)
            // 3-cost (8)
            ("chrome_signet", 2),     // chrome relic
            ("spellforge_engine", 2), // chrome relic
            ("adaptive_polyp", 2),    // bio 2/4 Evolve
            ("mycelium_weaver", 2),   // bio 2/2 growth
            // 4-cost (5)
            ("chrome_bulwark", 2),   // chrome 3/5 Drone lord + draw 2
            ("foundry_overseer", 1), // chrome Drone lord + draw on drone ETB
            ("thornveil_elder", 2),  // bio 3/4 growth
            // 5-6 cost finishers (5 — the missing top end)
            ("drone_convergence", 1),       // Mass Fabrication
            ("terravore", 1),               // Viral Assimilation
            ("chrome_battlesphere", 1),     // chrome 6-mana closer
            ("steel_titan", 1),             // chrome 6-mana closer
            ("corporate_reprogramming", 1), // Assimilate → Drone
        ],
        _ => vec![],
    }
}

/// Default 15-card sideboard for each stock mono-color deck. Empty for
/// multicolor/unknown decks. Players can fully edit this via the deck
/// builder, but this gives them a sensible starting pool for Bo3 instead
/// of an empty sideboard pane.
pub fn classic_default_sideboard_card_ids(deck_id: &str) -> Vec<(&'static str, i32)> {
    match deck_id {
        "neon" => vec![
            ("ambush_spike", 2),        // Flash Deathtouch — anti-aggro
            ("spell_shield", 1),        // life per instant/ritual — anti-burn
            ("packet_denial", 1),       // counter + draw
            ("resource_drain", 1),      // utility counter
            ("recursive_spike", 1),     // cheap bounce w/ Gravecast
            ("signal_blade", 1),        // weapon
            ("access_denied", 1),       // hard counter
            ("data_ripple", 2),         // instant tempo
            ("harbinger_of_static", 1), // flash bounce
            ("derez", 1),               // tempo bounce
            ("nullify", 1),             // exile anti-recursion
            ("dimensional_rift", 1),    // wipe 4+ toughness
            ("blink_of_a_cycle", 1),    // flicker protection
        ],
        "glitch" => vec![
            ("neon_spike", 2),       // 3/2 destroy-a-neon
            ("logic_bomb", 2),       // 2 damage + surveil
            ("victim_of_static", 1), // exile
            ("null_terminate", 1),   // destroy
            ("doom_code", 1),        // destroy + draw
            ("fatal_error", 1),      // 1-mana destroy
            ("entropy_spore", 2),    // cheap discard
            ("virus_disciple", 1),   // 2c utility
            ("final_shutdown", 1),   // anti-chrome
            ("shrieking_static", 1), // discard aura
            ("memetic_override", 1), // Assimilate->Virus
            ("archived_alive", 1),   // graveyard value
        ],
        "chrome" => vec![
            ("emergency_patch", 2),    // 1-mana indestructible
            ("deployment_pod", 2),     // 2 drone tokens
            ("reality_breach", 1),     // exile + draw
            ("chrome_sentinel", 1),    // Flash team-indestructible
            ("hardened_processor", 1), // 3/4 Camouflage+Vigilance
            ("fierce_barrier", 1),     // defensive tech
            ("chrome_aegis", 1),       // protection
            ("displacement_beam", 1),  // bounce
            ("exile_protocol", 1),     // exile
            ("shock_module", 1),       // 1 damage opp + draw
            ("chrome_scanner", 1),     // relic
            ("dispatch_protocol", 1),  // destroy + draw
            ("convergence_strike", 1), // burn
        ],
        "slag" => vec![
            ("chain_detonation", 1),       // mass -3/-3
            ("slag_storm", 1),             // -6/-6 single
            ("siege_behemoth", 1),         // 5/4 Trample+Haste
            ("forge_blast", 1),            // 5 damage + tokens
            ("meltdown_protocol", 2),      // wheel
            ("chrome_spike", 2),           // anti-chrome destroy
            ("magma_charger", 1),          // 2/2 Haste
            ("arc_discharge", 1),          // 2 damage
            ("jolt", 1),                   // 1 damage
            ("ember_phoenix", 1),          // recursive
            ("volatile_gremlin", 1),       // 1c haste
            ("bonfire_of_the_smelted", 1), // -5/-5 wipe
            ("reckless_salvage", 1),       // 3-mana ritual
        ],
        "bio" => vec![
            ("predatory_strike", 2),        // fight + pump
            ("bloom_burst", 1),             // 2 tokens + counters
            ("bane_of_circuitry", 1),       // destroy + token
            ("bramble_seedling", 2),        // anti-aggro blocker
            ("consumed_by_growth", 2),      // removal
            ("grove_huntmaster", 1),        // anthem
            ("champion_of_the_thicket", 1), // threat
            ("root_graft", 1),              // lifegain
            ("spore_shell", 1),             // defense
            ("fungal_ward", 1),             // protection
            ("overgrowth_veil", 1),         // untargetable
            ("deep_rooting", 1),            // two Spores
        ],
        _ => vec![],
    }
}

/// Public accessor — used by the AI sideboard handler to pick swaps that
/// mirror the gauntlet's proven matchup tech.
pub fn classic_sideboard_ins(deck_id: &str, opponent_deck_id: &str) -> Vec<(&'static str, i32)> {
    classic_sideboard_in_card_ids(deck_id, opponent_deck_id)
}

pub fn classic_sideboard_outs(deck_id: &str, opponent_deck_id: &str) -> Vec<(&'static str, i32)> {
    classic_sideboard_out_card_ids(deck_id, opponent_deck_id)
}

fn classic_sideboard_in_card_ids(
    deck_id: &str,
    opponent_deck_id: &str,
) -> Vec<(&'static str, i32)> {
    match (deck_id, opponent_deck_id) {
        ("bio", "neon") => vec![
            ("consumed_by_growth", 2),
            ("predatory_strike", 2),
            ("bane_of_circuitry", 2),
            ("champion_of_the_thicket", 2),
        ],
        ("bio", "slag") | ("bio", "rush") => vec![
            // Anti-aggro: more blockers + life gain
            ("bloom_warden", 2),
            ("bramble_seedling", 2),
            ("root_graft", 2),
        ],
        ("bio", "chrome") => vec![
            // Chrome has many artifacts + Drone tokens — bring wider board
            ("bloom_burst", 1),
            ("grove_huntmaster", 1),
            ("predatory_strike", 1),
        ],
        ("bio", "glitch") => vec![
            // Glitch removes 1-for-1 — bring 2-for-1s + token spam
            ("predatory_strike", 2),  // 2-mana fight + pump (2-for-1)
            ("bloom_burst", 2),       // 4-mana 2× 2/1 tokens
            ("bane_of_circuitry", 1), // 6-mana destroy + 2/1 token
        ],
        ("neon", "slag") | ("neon", "rush") => vec![
            // Counter slag's burn + sustain life
            ("ambush_spike", 2),
            ("spell_shield", 1),
            ("packet_denial", 1),
            ("resource_drain", 1),
            ("recursive_spike", 1),
            ("signal_blade", 1),
        ],
        ("neon", "takeover") => vec![
            // Takeover discards + assimilates — counter heavy
            ("access_denied", 2),
            ("data_ripple", 2),
        ],
        ("neon", "bio") => vec![
            // Neon loses to bio's slow value — bring counters + removal
            ("access_denied", 2),
            ("data_ripple", 2),
            ("harbinger_of_static", 2),
        ],
        ("neon", "fortress") => vec![
            // Fortress accumulates relics — disrupt early
            ("access_denied", 2),
            ("derez", 2),
        ],
        ("glitch", "neon") => vec![
            // Neon Stealth — hard counter + more removal
            ("neon_spike", 1),
            ("logic_bomb", 2),
            ("victim_of_static", 1),
        ],
        ("glitch", "takeover") => vec![
            // Mirror-ish — add extra removal and discard
            ("fatal_error", 2),
            ("doom_code", 1),
        ],
        ("glitch", "chrome") => vec![
            // Chrome ignores discard — go for removal + exile
            ("doom_code", 2),
            ("fatal_error", 2),
            ("final_shutdown", 1),
        ],
        ("glitch", "rush") => vec![
            // Rush races — need removal + blockers
            ("fatal_error", 2),
            ("logic_bomb", 2),
            ("entropy_spore", 1),
        ],
        ("chrome", "neon") => vec![
            // Stealth bypasses blocks — go ALL IN on removal + early pressure
            ("displacement_beam", 1),  // bumps to 3 — tempo bounce
            ("exile_protocol", 1),     // bumps to 3 — permanent removal
            ("chrome_sentinel", 1),    // bumps to 3 — Flash + team Indestructible
            ("hardened_processor", 1), // 3/4 Camouflage + Vigilance + draw
            ("emergency_patch", 2),    // 1-mana Indestructible + scry
            ("reality_breach", 1),     // 3-mana exile + draw
            ("deployment_pod", 1),     // 2-mana 2× 1/1 chump tokens
        ],
        ("slag", "chrome") => vec![("meltdown_protocol", 2), ("chain_detonation", 2)],
        ("slag", "fortress") => vec![("meltdown_protocol", 2), ("forge_blast", 2)],
        ("slag", "takeover") => vec![
            // Takeover is bio midrange — more burn to race
            ("arc_discharge", 1),
            ("magma_charger", 2),
            ("jolt", 1),
        ],
        ("slag", "bio") => vec![
            // Bio Interceptors need mass removal + trample
            ("chain_detonation", 1),
            ("slag_storm", 1),
            ("siege_behemoth", 1),
        ],
        ("slag", "glitch") => vec![
            // Glitch removal-heavy — more aggressive front
            ("volatile_gremlin", 1),
            ("magma_charger", 2),
        ],
        ("fortress", "rush") | ("fortress", "slag") => vec![
            // Fortress needs early answers vs aggro
            ("emergency_patch", 2),
            ("chrome_sentinel", 2),
        ],
        _ => vec![],
    }
}

fn classic_sideboard_out_card_ids(
    deck_id: &str,
    opponent_deck_id: &str,
) -> Vec<(&'static str, i32)> {
    match (deck_id, opponent_deck_id) {
        ("bio", "neon") => vec![
            ("primeval_colossus", 1),
            ("thornback_riders", 1),
            ("fauna_whisperer", 1),
            ("bloom_burst", 1),
            ("end_growth_forerunners", 1),
            ("verdant_zenith", 2),
            ("deep_rooting", 1),
        ],
        ("bio", "slag") | ("bio", "rush") => vec![
            // Cut slow finishers against aggro
            ("end_growth_forerunners", 1),
            ("terravore", 1),
            ("thornback_riders", 2),
            ("fauna_whisperer", 1),
        ],
        ("bio", "chrome") => vec![
            // Cut the chaff — keep finishers to grind chrome's long game
            ("spore_scout", 1),
            ("bloom_warden", 1),
            ("thornveil_elder", 1), // Interceptors wasted; chrome plays around combat
        ],
        ("bio", "glitch") => vec![
            ("spore_scout", 1),
            ("thornback_riders", 1),
            ("end_growth_forerunners", 1),
            ("verdant_zenith", 1),
            ("adaptive_polyp", 1), // loses value w/o combat (glitch just removes)
        ],
        ("neon", "slag") | ("neon", "rush") => vec![
            // Cut top-end + slow win-condition pieces vs aggro
            ("consecrated_server", 2),
            ("data_reroute", 1),
            ("master_of_signals", 1),
            ("lord_of_the_grid", 1),
            ("phase_skipper", 1),
        ],
        ("neon", "takeover") => vec![
            // Cut aggressive threats that get stolen
            ("phase_skipper", 1),
            ("stealth_infiltrator", 1),
        ],
        ("neon", "bio") => vec![
            // Cut slow top-end vs bio's grinding strategy
            ("consecrated_server", 2),
            ("data_reroute", 1),
            ("master_of_signals", 1),
            ("lord_of_the_grid", 2),
        ],
        ("neon", "fortress") => vec![
            // Cut top-end, need disruption
            ("consecrated_server", 2),
            ("lord_of_the_grid", 2),
        ],
        ("glitch", "neon") => vec![
            ("total_corruption", 1),
            ("archived_alive", 1),
            ("doom_code", 1),
            ("shrieking_static", 1), // discard dead vs fast neon
        ],
        ("glitch", "takeover") => vec![
            ("waste_not_want_not", 1),
            ("total_corruption", 1),
            ("archived_alive", 1),
        ],
        ("glitch", "chrome") => vec![
            ("archived_alive", 1),
            ("waste_not_want_not", 1),
            ("hymn_to_entropy", 1),
            ("memetic_override", 1),
            ("total_corruption", 1),
        ],
        ("glitch", "rush") => vec![
            ("total_corruption", 1),
            ("archived_alive", 1),
            ("doom_code", 1),
            ("hymn_to_entropy", 1),
            ("memetic_override", 1),
        ],
        ("chrome", "neon") => vec![
            ("corporate_reprogramming", 1),
            ("data_recovery_matrix", 1),
            ("convergence_strike", 1),
            ("spellforge_engine", 1),
            ("chrome_battlesphere", 1),
            ("overclock_matrix", 1),
            ("steel_titan", 1), // 6-mana value, too slow
        ],
        ("slag", "chrome") => vec![("jolt", 2), ("volatile_gremlin", 2)],
        ("slag", "fortress") => vec![("jolt", 2), ("slag_crawler", 2)],
        ("slag", "takeover") => vec![
            ("bonfire_of_the_smelted", 1),
            ("syr_konrad_the_molten", 2),
            ("blood_furnace_core", 1),
        ],
        ("slag", "bio") => vec![
            ("slag_crawler", 1),
            ("volatile_gremlin", 1),
            ("furnace_traitor", 1),
        ],
        ("slag", "glitch") => vec![
            ("bonfire_of_the_smelted", 1),
            ("syr_konrad_the_molten", 1),
            ("chain_detonation", 1),
        ],
        ("fortress", "rush") | ("fortress", "slag") => vec![
            // Cut the slowest value pieces vs aggro
            ("drone_convergence", 1),
            ("foundry_overseer", 1),
            ("corporate_reprogramming", 1),
            ("terravore", 1),
        ],
        _ => vec![],
    }
}

/// Returns the operator card IDs available for a given color.
/// Each entry is the card ID of an Apex operator.
pub fn operator_ids_for_color(deck_id: &str) -> Vec<&'static str> {
    match deck_id {
        "neon" => vec![
            // pure (5)
            "cipher_thief",
            "echo_signal",
            "grid_compiler",
            "cypher_data_architect",
            "chronos_temporal_hacker",
            // multi (2)
            "burnphase_infiltrator", // neon/slag
            "bioluminescent_oracle", // bio/neon
        ],
        "glitch" => vec![
            // pure (5)
            "bitrot_trinket_thief",
            "corruptor_of_data",
            "entropy_dreadhorde_general",
            "entropy_last_resort",
            "paradox_worm",
            // multi (2)
            "datavoid_phantom", // glitch/neon
            "entropy_pyre",     // glitch/slag
        ],
        "chrome" => vec![
            // pure (5)
            "urza_architect",
            "emry_salvager",
            "tezzeret_chrome",
            "karn_chrome",
            "bastion_prime",
            // multi (2)
            "rogue_protocol", // chrome/glitch
            "nexus_warden",   // chrome/neon
        ],
        "slag" => vec![
            // pure (5)
            "ignitor_god_of_the_forge",
            "pyra_torch_of_defiance",
            "pyra_awakened_inferno",
            "rivett_scrap_savant",
            "meltdown_titan",
            // multi (2)
            "wildfire_spirit", // bio/slag
            "forge_overseer",  // chrome/slag
        ],
        "bio" => vec![
            // pure (5)
            "xylo_operator",
            "symbiote_channeler",
            "sylva_voice_of_growth",
            "verdania_natures_fury",
            "briar_champion_of_wilds",
            // multi (2)
            "rootweaver_drone", // bio/chrome
            "rot_sporelord",    // bio/glitch
        ],
        _ => vec![],
    }
}

/// Builds a deck + operator card for simulation in Operator format.
/// Uses the same curated card list as classic, plus the specified operator from the catalog.
/// Returns (main_deck, operator_card).
pub fn build_operator_sim_deck(
    player_id: &str,
    deck_id: &str,
    operator_card_id: &str,
    seed: Option<u64>,
) -> Result<(Vec<Card>, Card)> {
    build_operator_sim_deck_with_patch(player_id, deck_id, operator_card_id, seed, None)
}

pub fn build_operator_sim_deck_with_patch(
    player_id: &str,
    deck_id: &str,
    operator_card_id: &str,
    seed: Option<u64>,
    patch: Option<&BalancePatch>,
) -> Result<(Vec<Card>, Card)> {
    const OPERATOR_POOL_SIZE: usize = 66;

    let mut all_cards = match deck_id {
        "neon" => card_catalog::neon::all_cards(),
        "glitch" => card_catalog::glitch::all_cards(),
        "chrome" => card_catalog::chrome::all_cards(),
        "slag" => card_catalog::slag::all_cards(),
        "bio" => card_catalog::bio::all_cards(),
        _ => return Err(anyhow!("Unknown deck color: {deck_id}")),
    };

    if let Some(patch) = patch {
        patch.apply_to_catalog_cards(&mut all_cards)?;
    }

    // Find the operator card in the catalog
    let operator_template = all_cards
        .iter()
        .find(|(c, _)| c.id == operator_card_id && c.supertype == CardSupertype::Apex)
        .map(|(c, _)| c.clone())
        .ok_or_else(|| anyhow!("Operator {operator_card_id} not found in {deck_id} catalog"))?;

    // Operator format is singleton: each unique non-Apex card appears once.
    // Take the first OPERATOR_POOL_SIZE non-Apex cards from the color catalog.
    let mut cards = Vec::new();
    let mut id_counter = 0usize;
    for (card, _) in &all_cards {
        if card.supertype == CardSupertype::Apex {
            continue;
        }
        if cards.len() >= OPERATOR_POOL_SIZE {
            break;
        }
        id_counter += 1;
        let id = format!("{}-{}-{}", player_id, card.id, id_counter);
        cards.push(clone_template(card, &id));
    }

    // Shuffle
    let mut rng = match seed {
        Some(base_seed) => {
            let mut hasher = DefaultHasher::new();
            player_id.hash(&mut hasher);
            let player_hash = hasher.finish();
            StdRng::seed_from_u64(base_seed ^ player_hash)
        }
        None => StdRng::seed_from_u64(rand::random::<u64>()),
    };
    shuffle_deck_with_rules(&mut cards, &mut rng);

    // Create operator instance
    id_counter += 1;
    let operator_id = format!("{}-{}-{}", player_id, operator_card_id, id_counter);
    let operator = clone_template(&operator_template, &operator_id);

    Ok((cards, operator))
}

pub fn build_deck_with_templates(
    player_id: &str,
    deck_id: &str,
    seed: Option<u64>,
    _templates: &DeckTemplates,
    opponent_deck_id: Option<&str>,
) -> Vec<Card> {
    build_deck_with_templates_and_patch(
        player_id,
        deck_id,
        seed,
        _templates,
        opponent_deck_id,
        None,
    )
    .unwrap_or_default()
}

pub fn build_deck_with_templates_and_patch(
    player_id: &str,
    deck_id: &str,
    seed: Option<u64>,
    _templates: &DeckTemplates,
    opponent_deck_id: Option<&str>,
    patch: Option<&BalancePatch>,
) -> Result<Vec<Card>> {
    // Load full card catalog for this deck. Mono-color decks load one
    // color; multi-color themed decks load both their colors.
    let mut all_cards = match deck_id {
        "neon" => card_catalog::neon::all_cards(),
        "glitch" => card_catalog::glitch::all_cards(),
        "chrome" => card_catalog::chrome::all_cards(),
        "slag" => card_catalog::slag::all_cards(),
        "bio" => card_catalog::bio::all_cards(),
        "rush" => {
            let mut v = card_catalog::slag::all_cards();
            v.extend(card_catalog::neon::all_cards());
            v
        }
        "takeover" => {
            let mut v = card_catalog::bio::all_cards();
            v.extend(card_catalog::glitch::all_cards());
            v
        }
        "fortress" => {
            let mut v = card_catalog::chrome::all_cards();
            v.extend(card_catalog::bio::all_cards());
            v
        }
        id if id.starts_with("syn_") => match card_catalog::syndicate::family_cards(id) {
            Some(v) => v,
            None => return Ok(Vec::new()),
        },
        _ => return Ok(Vec::new()),
    };
    let syndicate = deck_id.starts_with("syn_");
    if !syndicate {
        all_cards.extend(card_catalog::multicolor::all_cards());
    }

    if let Some(patch) = patch {
        patch.apply_to_catalog_cards(&mut all_cards)?;
    }

    // Get curated card IDs for classic format. Syndicate decks carry their
    // quantities on the catalog entries themselves.
    let mut curated_map: HashMap<String, i32> = if syndicate {
        all_cards
            .iter()
            .filter(|(c, qty)| *qty > 0 && c.supertype != CardSupertype::Apex)
            .map(|(c, qty)| (c.id.clone(), *qty))
            .collect()
    } else {
        classic_deck_card_ids(deck_id)
            .into_iter()
            .map(|(card_id, quantity)| (card_id.to_string(), quantity))
            .collect()
    };

    // Apply sideboard swaps if opponent is known
    if let Some(opponent_id) = opponent_deck_id {
        for (card_id, qty) in classic_sideboard_out_card_ids(deck_id, opponent_id) {
            if let Some(entry) = curated_map.get_mut(card_id) {
                *entry = (*entry - qty).max(0);
                if *entry == 0 {
                    curated_map.remove(card_id);
                }
            }
        }
        for (card_id, qty) in classic_sideboard_in_card_ids(deck_id, opponent_id) {
            *curated_map.entry(card_id.to_string()).or_insert(0) += qty;
        }
    }

    if let Some(patch) = patch {
        patch.apply_to_deck_quantities(deck_id, &mut curated_map)?;
    }

    // Build deck from catalog, filtering to curated cards.
    // Invariant: Apex cards are operator-only and must never appear in a
    // classic/non-operator deck (they would trivially dominate the format).
    let mut cards = Vec::new();
    let mut id_counter = 0usize;

    for (card, _operator_qty) in &all_cards {
        if let Some(&qty) = curated_map.get(card.id.as_str()) {
            if card.supertype == CardSupertype::Apex {
                return Err(anyhow!(
                    "Apex card '{}' is not allowed in non-operator deck '{}' — Apex is operator-format only",
                    card.id,
                    deck_id
                ));
            }
            for _ in 0..qty {
                id_counter += 1;
                let id = format!("{}-{}-{}", player_id, card.id, id_counter);
                cards.push(clone_template(card, &id));
            }
        }
    }

    // If no curated cards matched (card IDs don't exist yet), fall back to
    // selecting non-operator cards from the catalog automatically
    if cards.is_empty() {
        let non_operator: Vec<&Card> = all_cards
            .iter()
            .filter(|(c, _)| c.supertype != CardSupertype::Apex)
            .map(|(c, _)| c)
            .collect();

        let target_count = 34;
        for card in non_operator.iter().take(target_count) {
            id_counter += 1;
            let id = format!("{}-{}-{}", player_id, card.id, id_counter);
            cards.push(clone_template(card, &id));
        }
    }

    // Mana comes from the per-turn deploy_node action, not from dealt
    // infrastructure "land" cards.
    let _ = (player_id, &mut id_counter);

    let mut rng = match seed {
        Some(base_seed) => {
            let mut hasher = DefaultHasher::new();
            player_id.hash(&mut hasher);
            let player_hash = hasher.finish();
            StdRng::seed_from_u64(base_seed ^ player_hash)
        }
        None => StdRng::seed_from_u64(rand::random::<u64>()),
    };
    shuffle_deck_with_rules(&mut cards, &mut rng);
    Ok(cards)
}

pub fn build_deck(player_id: &str, deck_id: &str, seed: Option<u64>) -> Vec<Card> {
    build_deck_with_templates(player_id, deck_id, seed, &DeckTemplates::default(), None)
}

fn shuffle_deck_with_rules(cards: &mut Vec<Card>, rng: &mut StdRng) {
    cards.shuffle(rng);
}
