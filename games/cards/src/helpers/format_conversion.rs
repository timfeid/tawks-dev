//! Format conversion utilities between game and deck formats

use crate::types::{DeckFormat, GameFormat};

/// Converts a game format to its corresponding deck format
pub fn deck_format_for_game(format: &GameFormat) -> DeckFormat {
    match format {
        GameFormat::Operator => DeckFormat::Operator,
        GameFormat::Classic => DeckFormat::Classic,
    }
}

/// Converts a deck format to its corresponding game format
pub fn game_format_for_deck(format: &DeckFormat) -> GameFormat {
    match format {
        DeckFormat::Operator => GameFormat::Operator,
        DeckFormat::Classic => GameFormat::Classic,
    }
}
