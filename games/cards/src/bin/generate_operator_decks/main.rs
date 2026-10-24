mod bio;
mod chrome;
mod glitch;
mod multicolor;
mod neon;
mod slag;
mod synergy;

use anyhow::Result;
use cardgame::card::Card;
use cardgame::repository::CardRepository;
use cardgame::types::DeckFormat;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::sync::Arc;

fn strip_punctuation(s: &str) -> String {
    s.chars()
        .filter(|c| !c.is_ascii_punctuation())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn compare_descriptions(cards: &[(Card, i32)]) {
    println!("\n=== Comparing Manual vs Auto-Generated Descriptions ===\n");

    let mut differences = 0;
    let mut matches = 0;

    for (card, _) in cards {
        let manual = strip_punctuation(&card.description);
        let auto = strip_punctuation(&card.auto_description);
        if manual != auto {
            differences += 1;
            println!("DIFF  {}: {}", card.name, card.description);
            println!("      {}: {}", card.name, card.auto_description);
        } else {
            matches += 1;
            println!("  OK  {}: {}", card.name, card.description);
        }
    }

    println!(
        "Summary: {} matches, {} differences\n",
        matches, differences
    );
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();
    let _ = dotenvy::dotenv();

    let decks: Vec<(&str, &str, &str, Option<&str>, Vec<(Card, i32)>)> = vec![
        (
            "roots_of_infinity",
            "Roots of Infinity",
            "This deck achieves exponential growth through stacking anthem effects, token generation, and evasive threats. With 6 anthem lords granting +1/+1 to Bio-Organisms, your token army quickly grows massive. Mana-scaling units tap for Bio mana equal to your unit count, fueling explosive turns. Small evasion units like Spore Scout and Toxic Stalker carry anthem buffs through unblocked, while protection instants (Fungal Ward, Spore Shell, Overgrowth Veil) keep key pieces alive. Graveyard recursion via Root Memory and Mycorrhizal Recall recovers your best threats after board wipes. Finishers like Apex Stampede turn your wide board into a lethal alpha strike.",
            Some("https://cdn.tawks.app/decks/roots_of_infinity_1772760842.png"),
            bio::generate_deck(),
        ),
        (
            "flicker_strike",
            "Flicker Strike",
            "A tempo-focused deck that leverages unblockable units and instant-speed interaction. With 8 anthem lords buffing your Drone units and over 15 unblockable threats, you'll consistently deal damage while drawing cards through combat damage triggers. The deck runs 12 Flash units, allowing you to develop your board on opponents' turns while holding up interaction. Bounce effects let you reuse ETB triggers and disrupt opponent strategies. The synergy is tight: unblockable units guarantee damage triggers, card draw fuels more threats, Flash units enable double-spelling, and anthem effects turn evasive 1/1s into serious threats.",
            Some("https://cdn.tawks.app/decks/flicker_strike_1772760794.png"),
            neon::generate_deck(),
        ),
        (
            "mind_rot",
            "Mind Rot",
            "A control deck built around resource denial and hand disruption. Multiple discard engines force opponents to discard cards, while payoff cards like Entropy's Caress and Data Hemorrhage convert each discarded card into direct damage. Hand-punishment effects like Shrieking Static and The Vice deal escalating damage as opponents' hands empty. Wheel effects refill your hand while forcing opponents to discard their carefully-held answers. The deck grinds opponents down through inevitability: they can't keep cards in hand, they take damage for having empty hands, and recursive threats keep coming back from your graveyard.",
            Some("https://cdn.tawks.app/decks/mind_rot_1772760817.png"),
            glitch::generate_deck(),
        ),
        (
            "blood_furnace",
            "Blood Furnace",
            "A sacrifice-focused deck where units dying triggers devastating effects. Death-trigger payoffs like Siphon Welder, Exhaust Cutthroat, and Mayhem Forgefiend stack multiplicatively—each unit death deals 3+ damage across all three triggers. Multiple sacrifice outlets let you convert units into value at instant speed. Recursive units return from the graveyard infinitely, providing endless fodder. The synergy creates a drain engine: sacrifice units to generate value, death triggers deal massive damage to opponents, recursive threats come back for free, and the cycle continues until opponents are eliminated through sheer inevitability.",
            Some("https://cdn.tawks.app/decks/blood_furnace_1772760782.png"),
            slag::generate_deck(),
        ),
        (
            "forge_protocol",
            "Forge Protocol",
            "An artifact-based protection deck that dominates through mana advantage and resilience. With Vyktor, Master Architect transforming every artifact into Chrome mana, even a modest board of 6 artifacts taps for 6 mana per turn. Tax effects like Throttle Grid slow opponents down while protection instants keep your board intact. Artifact tutors find any piece you need, and the Parasitic Scanner taxes every opponent spell into a free draw. Small evasive Drones like Chrome Phantom carry card draw triggers through unblocked. The deck scales exponentially: more artifacts means more mana, more mana enables more artifacts, and finishers like Forge Colossus and Core Assembler Engine close out games while staying ahead on cards and board.",
            Some("https://cdn.tawks.app/decks/forge_protocol_1772760805.png"),
            chrome::generate_deck(),
        ),
    ];

    // Compare manual vs auto-generated descriptions
    for (_, deck_name, _, _, cards) in &decks {
        println!("\n=== Deck: {} ===", deck_name);
        compare_descriptions(cards);
    }

    let database_url =
        env::var("CARDGAME_DATABASE_URL").expect("CARDGAME_DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;
    let repo = Arc::new(CardRepository::new(Arc::new(pool)));

    // Seed ALL cards from every color catalog into the DB
    let all_catalog_cards: Vec<Vec<(Card, i32)>> = vec![
        cardgame::card_catalog::bio::all_cards(),
        cardgame::card_catalog::chrome::all_cards(),
        cardgame::card_catalog::glitch::all_cards(),
        cardgame::card_catalog::neon::all_cards(),
        cardgame::card_catalog::slag::all_cards(),
        cardgame::card_catalog::multicolor::all_cards(),
    ];
    let mut catalog_count = 0;
    for catalog in &all_catalog_cards {
        for (card, _) in catalog {
            repo.create_card(card).await?;
            catalog_count += 1;
        }
    }
    println!("✓ Seeded {} catalog cards from all colors!", catalog_count);

    for (deck_id, deck_name, desc, image_url, cards) in &decks {
        for (card, _) in cards {
            repo.create_card(card).await?;
        }
        sqlx::query("DELETE FROM decks WHERE deck_id = $1")
            .bind(deck_id)
            .execute(&*repo.pool())
            .await?;
        create_deck(&repo, deck_id, deck_name, desc, *image_url, cards).await?;
        println!("✓ Generated '{}' Operator deck!", deck_name);
    }

    // Register multicolor combo cards (available as expansion/pack cards)
    let multicolor_cards = multicolor::generate_multicolor_cards();
    for (card, _) in &multicolor_cards {
        repo.create_card(card).await?;
    }
    println!(
        "✓ Registered {} multicolor combo cards!",
        multicolor_cards.len()
    );

    // Register synergy expansion cards (50 mono + 50 multicolor)
    let synergy_cards = synergy::generate_synergy_cards();
    for (card, _) in &synergy_cards {
        repo.create_card(card).await?;
    }
    println!(
        "✓ Registered {} synergy expansion cards!",
        synergy_cards.len()
    );

    // Seed classic base decks (same 5 colors, classic format, no operator)
    let catalog = cardgame::deck::build_deck_catalog();
    repo.seed_deck_catalog(&catalog).await?;
    println!("✓ Seeded {} classic base decks!", catalog.len());

    // Collect every canonical card ID, then delete stale cards not in the set
    let mut all_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    for catalog in &all_catalog_cards {
        for (card, _) in catalog {
            all_ids.insert(card.id.clone());
        }
    }
    for (_, _, _, _, cards) in &decks {
        for (card, _) in cards {
            all_ids.insert(card.id.clone());
        }
    }
    for (card, _) in &multicolor_cards {
        all_ids.insert(card.id.clone());
    }
    for (card, _) in &synergy_cards {
        all_ids.insert(card.id.clone());
    }

    let all_ids_vec: Vec<String> = all_ids.into_iter().collect();
    let placeholders: Vec<String> = all_ids_vec
        .iter()
        .enumerate()
        .map(|(i, _)| format!("${}", i + 1))
        .collect();
    let delete_sql = format!(
        "DELETE FROM cards WHERE card_id NOT IN ({})",
        placeholders.join(", ")
    );
    let mut query = sqlx::query(&delete_sql);
    for id in &all_ids_vec {
        query = query.bind(id);
    }
    let result = query.execute(&*repo.pool()).await?;
    let deleted = result.rows_affected();
    if deleted > 0 {
        println!("✓ Removed {} stale cards from DB", deleted);
    }
    println!("✓ Done! {} canonical cards in DB", all_ids_vec.len());

    Ok(())
}

async fn create_deck(
    repo: &Arc<CardRepository>,
    deck_id: &str,
    name: &str,
    desc: &str,
    image_url: Option<&str>,
    cards: &[(cardgame::card::Card, i32)],
) -> Result<()> {
    let operator_id = cards[0].0.id.clone();
    repo.create_deck_with_owner(deck_id, name, desc, None, None)
        .await?;
    if let Some(url) = image_url {
        repo.update_deck_image_url(deck_id, url).await?;
    }
    sqlx::query("DELETE FROM deck_cards WHERE deck_id = $1")
        .bind(deck_id)
        .execute(&*repo.pool())
        .await?;
    repo.set_deck_format(deck_id, DeckFormat::Operator).await?;
    repo.set_deck_operator(deck_id, Some(&operator_id)).await?;

    // Operator format: 66 cards + 1 operator = 67 template entries.
    // Seed exactly 67 cards: the operator + 66 others.
    let mut non_operator_count = 0;
    for (card, quantity) in cards {
        if card.id == operator_id {
            repo.adjust_deck_card_quantity(deck_id, &card.id, *quantity)
                .await?;
        } else if non_operator_count < 66 {
            repo.adjust_deck_card_quantity(deck_id, &card.id, *quantity)
                .await?;
            non_operator_count += quantity;
        }
    }
    Ok(())
}
