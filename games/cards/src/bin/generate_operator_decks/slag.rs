use cardgame::card::Card;

pub fn generate_deck() -> Vec<(Card, i32)> {
    cardgame::card_catalog::slag::all_cards()
}
