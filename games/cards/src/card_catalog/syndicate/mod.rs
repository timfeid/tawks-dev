//! Syndicate: a crime card game on the Mana Clash engine. Cards cost cash
//! (`ManaCost.colorless`), colours mark the family, rackets are
//! `CardKind::Infrastructure` with a turn-start `AddMana(Colorless, n)` trigger.

use crate::card::{Card, CardBuilder, CardEffect, CardTrigger};
use crate::deck::DeckDefinition;
use crate::types::{CardKind, ManaCost, ManaType, TriggerEvent, TriggerLimit, UnitType};

pub mod cartel;
pub mod firm;
pub mod neutral;
pub mod old_blood;
pub mod street;
pub mod wolves;

pub const FAMILY_DECKS: &[&str] = &[
    "syn_old_blood",
    "syn_street",
    "syn_cartel",
    "syn_firm",
    "syn_wolves",
];

pub fn deck_definitions() -> Vec<DeckDefinition> {
    vec![
        DeckDefinition {
            id: "syn_old_blood",
            name: "Old Blood",
            description: "Syndicate. Made men, protection, and punishing anyone who comes at the family.",
        },
        DeckDefinition {
            id: "syn_street",
            name: "Street",
            description: "Syndicate. Cheap crews that go wide and fast, and run hot doing it.",
        },
        DeckDefinition {
            id: "syn_cartel",
            name: "Cartel",
            description: "Syndicate. Rackets that print cash, then bombs nobody else can afford.",
        },
        DeckDefinition {
            id: "syn_firm",
            name: "The Firm",
            description: "Syndicate. Lawyers, hackers and fixers. Cool heat, steal cash, control the board.",
        },
        DeckDefinition {
            id: "syn_wolves",
            name: "Wolves",
            description: "Syndicate. Sacrifice, recursion, and crews that thrive in debt.",
        },
    ]
}

/// Family cards plus the neutral pool shared by every family.
pub fn family_cards(deck_id: &str) -> Option<Vec<(Card, i32)>> {
    let mut cards = match deck_id {
        "syn_old_blood" => old_blood::all_cards(),
        "syn_street" => street::all_cards(),
        "syn_cartel" => cartel::all_cards(),
        "syn_firm" => firm::all_cards(),
        "syn_wolves" => wolves::all_cards(),
        _ => return None,
    };
    cards.extend(neutral::all_cards());
    Some(cards)
}

/// Every Syndicate card once, for catalog-wide checks.
pub fn all_cards() -> Vec<(Card, i32)> {
    let mut v = old_blood::all_cards();
    v.extend(street::all_cards());
    v.extend(cartel::all_cards());
    v.extend(firm::all_cards());
    v.extend(wolves::all_cards());
    v.extend(neutral::all_cards());
    v
}

/// Family colour names, reusing the Mana Clash palette for frames and deck identity.
pub const OLD_BLOOD: &str = "chrome";
pub const STREET: &str = "neon";
pub const CARTEL: &str = "bio";
pub const FIRM: &str = "glitch";
pub const WOLVES: &str = "slag";

pub fn cash(n: i32) -> ManaCost {
    ManaCost {
        colorless: n,
        ..Default::default()
    }
}

pub fn on(event: TriggerEvent, effect: CardEffect) -> CardTrigger {
    CardTrigger {
        event,
        effect,
        limit: TriggerLimit::Unlimited,
        predicate: None,
        optional_cost: None,
        is_optional: false,
    }
}

/// A racket: permanent that pays `income` cash at the start of its owner's turn.
pub fn racket(id: &str, name: &str, cost: i32, income: i32, family: &str) -> CardBuilder {
    CardBuilder::new(id, name, CardKind::Infrastructure)
        .cost(cash(cost))
        .colors(&[family])
        .trigger(on(
            TriggerEvent::TurnStart,
            CardEffect::AddMana(ManaType::Colorless, income),
        ))
}

pub fn crew(id: &str, name: &str, cost: i32, power: i32, toughness: i32, family: &str) -> CardBuilder {
    CardBuilder::new(id, name, CardKind::Unit(UnitType::Human))
        .cost(cash(cost))
        .stats(power, toughness)
        .colors(&[family])
}

/// A job: sorcery-speed one-shot.
pub fn job(id: &str, name: &str, cost: i32, family: &str) -> CardBuilder {
    CardBuilder::new(id, name, CardKind::Ritual)
        .cost(cash(cost))
        .colors(&[family])
}

/// A trick: instant-speed one-shot.
pub fn trick(id: &str, name: &str, cost: i32, family: &str) -> CardBuilder {
    CardBuilder::new(id, name, CardKind::Instant)
        .cost(cash(cost))
        .colors(&[family])
}
