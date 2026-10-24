use crate::card::Card;
use crate::state::{CardGameState, PlayerState};
use rand::{SeedableRng, seq::SliceRandom};
use rand_chacha::ChaCha8Rng;
use std::collections::{HashMap, HashSet};

/// Determinize hidden zones for search-based agents while preserving the root
/// player's currently visible information and prompt legality.
pub fn determinize_state(state: &CardGameState, player_id: &str, seed: u64) -> CardGameState {
    let mut determinized = state.clone();
    let mut rng = ChaCha8Rng::seed_from_u64(seed);

    for current_player_id in determinization_order(state) {
        let Some(player) = determinized.players.get_mut(&current_player_id) else {
            continue;
        };

        if current_player_id == player_id {
            determinize_root_player(player, &mut rng);
        } else {
            determinize_hidden_player(state, player, &mut rng);
        }
    }

    determinized
}

fn determinization_order(state: &CardGameState) -> Vec<String> {
    let mut ordered = state.turn_order.clone();
    let known: HashSet<_> = ordered.iter().cloned().collect();
    let mut extras = state
        .players
        .keys()
        .filter(|id| !known.contains(*id))
        .cloned()
        .collect::<Vec<_>>();
    extras.sort();
    ordered.extend(extras);
    ordered
}

fn determinize_root_player(player: &mut PlayerState, rng: &mut ChaCha8Rng) {
    if player.library_search_active {
        player.deck.shuffle(rng);
        return;
    }

    if !player.scry.is_empty() {
        preserve_known_scry_top(player, rng);
        return;
    }

    player.deck.shuffle(rng);
}

fn determinize_hidden_player(
    state: &CardGameState,
    player: &mut PlayerState,
    rng: &mut ChaCha8Rng,
) {
    let hand_size = player.hand.len();
    let scry_len = player.scry.len();
    let public_graveyard_search =
        player.library_search_active && is_public_graveyard_search(state, &player.scry);

    sample_hidden_hand_and_deck(player, hand_size, rng);

    if player.library_search_active {
        if !public_graveyard_search {
            player.scry = sample_search_results_from_deck(&player.deck, scry_len, rng);
        }
        return;
    }

    if scry_len > 0 {
        player.scry = scry_view_from_deck(&player.deck, scry_len);
    }
}

fn sample_hidden_hand_and_deck(player: &mut PlayerState, hand_size: usize, rng: &mut ChaCha8Rng) {
    let mut hidden_cards = std::mem::take(&mut player.hand);
    hidden_cards.extend(std::mem::take(&mut player.deck));
    hidden_cards.shuffle(rng);

    let hand_size = hand_size.min(hidden_cards.len());
    player.hand = hidden_cards.drain(..hand_size).collect();
    player.deck = hidden_cards;
}

fn preserve_known_scry_top(player: &mut PlayerState, rng: &mut ChaCha8Rng) {
    let known_ids: HashSet<_> = player.scry.iter().map(|card| card.id.clone()).collect();
    let mut known_cards = HashMap::new();
    let mut hidden_cards = Vec::new();

    for card in std::mem::take(&mut player.deck) {
        if known_ids.contains(&card.id) {
            known_cards.insert(card.id.clone(), card);
        } else {
            hidden_cards.push(card);
        }
    }

    hidden_cards.shuffle(rng);

    for card in player.scry.iter().rev() {
        if let Some(card) = known_cards.remove(&card.id) {
            hidden_cards.push(card);
        }
    }

    player.deck = hidden_cards;
}

fn sample_search_results_from_deck(deck: &[Card], count: usize, rng: &mut ChaCha8Rng) -> Vec<Card> {
    let mut candidates = deck.to_vec();
    candidates.shuffle(rng);
    candidates.truncate(count.min(candidates.len()));
    candidates
}

fn scry_view_from_deck(deck: &[Card], count: usize) -> Vec<Card> {
    let count = count.min(deck.len());
    let start = deck.len().saturating_sub(count);
    let mut scry = deck[start..].to_vec();
    scry.reverse();
    scry
}

fn is_public_graveyard_search(state: &CardGameState, scry: &[Card]) -> bool {
    !scry.is_empty()
        && scry.iter().all(|card| {
            state.players.values().any(|player| {
                player
                    .graveyard
                    .iter()
                    .any(|graveyard_card| graveyard_card.id == card.id)
            })
        })
}

#[cfg(test)]
mod tests {
    use super::{determinize_state, scry_view_from_deck};
    use crate::card::{Card, CardBuilder};
    use crate::state::{CardGameState, PlayerState};
    use crate::types::CardKind;
    use std::collections::BTreeSet;

    fn make_card(id: &str) -> Card {
        CardBuilder::new(id, id, CardKind::Ritual).build()
    }

    fn make_player(id: &str) -> PlayerState {
        PlayerState::new(
            id.to_string(),
            id.to_string(),
            String::new(),
            format!("{id}-instance"),
            20,
        )
    }

    fn make_state() -> CardGameState {
        let mut state = CardGameState::default();
        state.turn_order = vec!["p1".into(), "p2".into()];
        state.active_player_id = Some("p1".into());
        state.players.insert("p1".into(), make_player("p1"));
        state.players.insert("p2".into(), make_player("p2"));
        state
    }

    fn ids(cards: &[Card]) -> Vec<String> {
        cards.iter().map(|card| card.id.clone()).collect()
    }

    fn hidden_snapshot(
        state: &CardGameState,
    ) -> Vec<(String, Vec<String>, Vec<String>, Vec<String>)> {
        state
            .turn_order
            .iter()
            .map(|player_id| {
                let player = state.players.get(player_id).unwrap();
                (
                    player_id.clone(),
                    ids(&player.hand),
                    ids(&player.deck),
                    ids(&player.scry),
                )
            })
            .collect()
    }

    fn hidden_pool(player: &PlayerState) -> BTreeSet<String> {
        player
            .hand
            .iter()
            .chain(player.deck.iter())
            .map(|card| card.id.clone())
            .collect()
    }

    #[test]
    fn determinize_same_seed_is_stable_and_preserves_root_visible_information() {
        let mut state = make_state();

        {
            let root = state.players.get_mut("p1").unwrap();
            root.hand = vec![make_card("root_hand")];
            root.deck = vec![
                make_card("root_a"),
                make_card("root_b"),
                make_card("root_c"),
                make_card("root_d"),
            ];
            root.scry = vec![make_card("root_d"), make_card("root_c")];
        }

        {
            let opponent = state.players.get_mut("p2").unwrap();
            opponent.hand = vec![make_card("opp_hand_1"), make_card("opp_hand_2")];
            opponent.deck = vec![
                make_card("opp_a"),
                make_card("opp_b"),
                make_card("opp_c"),
                make_card("opp_d"),
            ];
        }

        let first = determinize_state(&state, "p1", 77);
        let second = determinize_state(&state, "p1", 77);

        assert_eq!(hidden_snapshot(&first), hidden_snapshot(&second));

        let root = first.players.get("p1").unwrap();
        assert_eq!(ids(&root.hand), vec!["root_hand"]);
        assert_eq!(ids(&root.scry), vec!["root_d", "root_c"]);

        let deck_ids = ids(&root.deck);
        assert_eq!(deck_ids[deck_ids.len() - 2..], ["root_c", "root_d"]);
        assert_eq!(
            deck_ids[..deck_ids.len() - 2]
                .iter()
                .cloned()
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["root_a".to_string(), "root_b".to_string()])
        );
    }

    #[test]
    fn determinize_resamples_hidden_opponent_hand_and_derives_plain_scry_from_deck_top() {
        let mut state = make_state();

        {
            let opponent = state.players.get_mut("p2").unwrap();
            opponent.hand = vec![make_card("opp_hand_1"), make_card("opp_hand_2")];
            opponent.deck = vec![
                make_card("opp_a"),
                make_card("opp_b"),
                make_card("opp_c"),
                make_card("opp_d"),
                make_card("opp_e"),
                make_card("opp_f"),
            ];
            opponent.scry = vec![make_card("opp_f"), make_card("opp_e"), make_card("opp_d")];
        }

        let first = determinize_state(&state, "p1", 1);
        let second = determinize_state(&state, "p1", 2);

        let original_hidden = hidden_pool(state.players.get("p2").unwrap());
        let first_opponent = first.players.get("p2").unwrap();
        let second_opponent = second.players.get("p2").unwrap();

        assert_eq!(hidden_pool(first_opponent), original_hidden);
        assert_eq!(hidden_pool(second_opponent), original_hidden);
        assert_eq!(
            ids(&first_opponent.scry),
            ids(&scry_view_from_deck(&first_opponent.deck, 3))
        );
        assert_eq!(
            ids(&second_opponent.scry),
            ids(&scry_view_from_deck(&second_opponent.deck, 3))
        );
        assert_ne!(
            (
                ids(&first_opponent.hand),
                ids(&first_opponent.deck),
                ids(&first_opponent.scry)
            ),
            (
                ids(&second_opponent.hand),
                ids(&second_opponent.deck),
                ids(&second_opponent.scry)
            )
        );
    }

    #[test]
    fn determinize_preserves_public_graveyard_search_results() {
        let mut state = make_state();

        {
            let root = state.players.get_mut("p1").unwrap();
            root.graveyard = vec![make_card("grave_hit")];
        }

        {
            let opponent = state.players.get_mut("p2").unwrap();
            opponent.hand = vec![make_card("opp_hand_1")];
            opponent.deck = vec![
                make_card("opp_a"),
                make_card("opp_b"),
                make_card("opp_c"),
                make_card("opp_d"),
            ];
            opponent.scry = vec![make_card("grave_hit")];
            opponent.library_search_active = true;
        }

        let determinized = determinize_state(&state, "p1", 19);
        let opponent = determinized.players.get("p2").unwrap();

        assert_eq!(ids(&opponent.scry), vec!["grave_hit"]);
        assert!(
            determinized
                .players
                .get("p1")
                .unwrap()
                .graveyard
                .iter()
                .any(|card| card.id == "grave_hit")
        );
        assert!(!opponent.deck.iter().any(|card| card.id == "grave_hit"));
    }

    #[test]
    fn determinize_preserves_root_search_results_in_deck() {
        let mut state = make_state();

        {
            let root = state.players.get_mut("p1").unwrap();
            root.deck = vec![
                make_card("root_a"),
                make_card("root_match_1"),
                make_card("root_b"),
                make_card("root_match_2"),
            ];
            root.scry = vec![make_card("root_match_1"), make_card("root_match_2")];
            root.library_search_active = true;
        }

        let determinized = determinize_state(&state, "p1", 23);
        let root = determinized.players.get("p1").unwrap();

        assert_eq!(ids(&root.scry), vec!["root_match_1", "root_match_2"]);
        assert!(root.deck.iter().any(|card| card.id == "root_match_1"));
        assert!(root.deck.iter().any(|card| card.id == "root_match_2"));
    }
}
