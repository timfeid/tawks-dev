//! Payouts for finished games, sent to Tawks as `game_award`. Tawks owns the
//! amounts and caps; the game only reports who finished where.

use std::collections::HashSet;

use serde_json::{Value, json};

use crate::room::{FinishedTable, PlayedBout};

pub const PARTY_GAME: &str = "tawkathon";
pub const BOUT_GAME: &str = "tawkathon-bout";
const MIN_PEOPLE: usize = 2;

/// `game_award` arguments, or nothing when fewer than two people played.
/// Each person counts once, at their best place; empty groups are dropped.
pub fn award(
    game: &str,
    reference_id: String,
    lobby_id: &str,
    placements: Vec<Vec<String>>,
    unplaced: Vec<String>,
    meta: Value,
) -> Option<Value> {
    let mut seen = HashSet::new();
    let placements: Vec<Vec<String>> = placements
        .into_iter()
        .map(|group| group.into_iter().filter(|u| seen.insert(u.clone())).collect::<Vec<_>>())
        .filter(|group| !group.is_empty())
        .collect();
    let players: Vec<Value> = unplaced
        .into_iter()
        .filter(|u| seen.insert(u.clone()))
        .map(|u| json!({ "user_id": u }))
        .collect();
    if seen.len() < MIN_PEOPLE {
        return None;
    }
    Some(json!({
        "game": game,
        "reference_id": reference_id,
        "lobby_id": lobby_id,
        "placements": placements,
        "players": players,
        "meta": meta,
    }))
}

/// One bout. Speedball played on its own pays as Speedball, winners only
/// placed; every other bout, and Speedball inside a party, is a Tawkathon bout.
pub fn bout(party_id: &str, lobby_id: &str, played: &PlayedBout, guests: &HashSet<String>) -> Option<Value> {
    let speedball = played.round == 0 && played.key == crate::speedball::KEY;
    let game = if speedball { crate::speedball::KEY } else { BOUT_GAME };
    let people: Vec<&(String, usize)> = played.players.iter().filter(|(u, _)| !guests.contains(u)).collect();
    let placed = |place: usize| place > 0 && (!speedball || place == 1);
    let mut places: Vec<usize> = people.iter().map(|(_, p)| *p).filter(|p| placed(*p)).collect();
    places.sort_unstable();
    places.dedup();
    let placements = places
        .iter()
        .map(|place| people.iter().filter(|(_, p)| p == place).map(|(u, _)| u.clone()).collect())
        .collect();
    let unplaced = people.iter().filter(|(_, p)| !placed(*p)).map(|(u, _)| u.clone()).collect();
    award(
        game,
        format!("{game}:{party_id}:{}", played.number),
        lobby_id,
        placements,
        unplaced,
        json!({ "bout": played.key, "round": played.round }),
    )
}

/// A whole Tawkathon, by final standings of seats. `seated` is each signed-in
/// person who played, by seat.
pub fn party(party_id: &str, lobby_id: &str, standings: &[Vec<u8>], seated: &[(u8, String)]) -> Option<Value> {
    let placements = standings
        .iter()
        .map(|group| seated.iter().filter(|(s, _)| group.contains(s)).map(|(_, u)| u.clone()).collect())
        .collect();
    let unplaced = seated.iter().map(|(_, u)| u.clone()).collect();
    award(PARTY_GAME, format!("{PARTY_GAME}:{party_id}"), lobby_id, placements, unplaced, Value::Null)
}

/// A table game: its winners first, everyone else who played unplaced.
pub fn table(lobby_id: &str, finished: &FinishedTable, guests: &HashSet<String>) -> Option<Value> {
    let keep = |users: &[String]| users.iter().filter(|u| !guests.contains(*u)).cloned().collect::<Vec<_>>();
    award(
        finished.key,
        format!("{}:{}", finished.key, finished.id),
        lobby_id,
        vec![keep(&finished.winners)],
        keep(&finished.others),
        Value::Null,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn played(key: &'static str, round: u32, players: &[(&str, usize)]) -> PlayedBout {
        PlayedBout {
            number: 3,
            key,
            round,
            players: players.iter().map(|(u, p)| (u.to_string(), *p)).collect(),
        }
    }

    fn ids(value: &Value) -> Vec<String> {
        value["players"].as_array().unwrap().iter().map(|p| p["user_id"].as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn a_bout_groups_ties_and_leaves_guests_and_unplaced_out_of_placements() {
        let guests = HashSet::from(["g".to_string()]);
        let bout = played("sumo", 2, &[("c", 3), ("a", 1), ("b", 1), ("g", 2), ("d", 0)]);
        let value = super::bout("party-1", "lobby", &bout, &guests).unwrap();
        assert_eq!(value["game"], BOUT_GAME);
        assert_eq!(value["reference_id"], "tawkathon-bout:party-1:3");
        assert_eq!(value["lobby_id"], "lobby");
        assert_eq!(value["placements"], json!([["a", "b"], ["c"]]));
        assert_eq!(ids(&value), vec!["d"]);
    }

    #[test]
    fn speedball_on_its_own_places_only_the_winners() {
        let none = HashSet::new();
        let bout = played(crate::speedball::KEY, 0, &[("a", 1), ("b", 2), ("c", 1)]);
        let value = super::bout("p", "l", &bout, &none).unwrap();
        assert_eq!(value["game"], "speedball");
        assert_eq!(value["reference_id"], "speedball:p:3");
        assert_eq!(value["placements"], json!([["a", "c"]]));
        assert_eq!(ids(&value), vec!["b"]);
        let in_party = played(crate::speedball::KEY, 1, &[("a", 1), ("b", 2)]);
        assert_eq!(super::bout("p", "l", &in_party, &none).unwrap()["game"], BOUT_GAME);
    }

    #[test]
    fn fewer_than_two_people_pays_nothing() {
        let guests = HashSet::from(["g".to_string()]);
        assert!(super::bout("p", "l", &played("sumo", 0, &[("a", 1), ("g", 2)]), &guests).is_none());
    }

    #[test]
    fn a_party_maps_seats_to_people_and_skips_seats_without_one() {
        let seated = vec![(0, "a".to_string()), (2, "c".to_string()), (3, "d".to_string())];
        let standings = vec![vec![2], vec![0, 1], vec![4]];
        let value = party("p", "l", &standings, &seated).unwrap();
        assert_eq!(value["game"], PARTY_GAME);
        assert_eq!(value["reference_id"], "tawkathon:p");
        assert_eq!(value["placements"], json!([["c"], ["a"]]));
        assert_eq!(ids(&value), vec!["d"]);
    }

    #[test]
    fn a_room_records_each_finished_table_game_by_person_without_bots() {
        use crate::room::{Phase, Room};
        let mut room = Room::new("table-room", 3);
        room.connect("a", "A", "").unwrap();
        room.connect("b", "B", "").unwrap();
        room.choose_game("a", crate::landlord::KEY).unwrap();
        room.add_bot("a").unwrap();
        let mut ids = Vec::new();
        for _ in 0..2 {
            room.start("a").unwrap();
            room.disconnect("a");
            room.disconnect("b");
            for _ in 0..60 * 60 * 60 {
                room.step();
                if matches!(room.phase, Phase::GameOver { .. }) {
                    break;
                }
            }
            assert!(matches!(room.phase, Phase::GameOver { .. }), "the bots finish the game");
            let finished = room.last_table.clone().unwrap();
            assert_eq!(finished.key, crate::landlord::KEY);
            let mut people: Vec<String> = finished.winners.iter().chain(&finished.others).cloned().collect();
            people.sort();
            assert_eq!(people, vec!["a", "b"]);
            ids.push(finished.id);
            while room.phase != Phase::Lobby {
                room.step();
            }
            room.connect("a", "A", "").unwrap();
            room.connect("b", "B", "").unwrap();
        }
        assert_ne!(ids[0], ids[1], "a rematch is a new game");
    }

    #[test]
    fn a_table_game_puts_winners_first_and_the_rest_unplaced() {
        let guests = HashSet::from(["g".to_string()]);
        let finished = FinishedTable {
            id: "lobby:2:99".into(),
            key: "coven",
            winners: vec!["a".into(), "g".into()],
            others: vec!["b".into(), "c".into()],
        };
        let value = table("lobby", &finished, &guests).unwrap();
        assert_eq!(value["game"], "coven");
        assert_eq!(value["reference_id"], "coven:lobby:2:99");
        assert_eq!(value["placements"], json!([["a"]]));
        assert_eq!(ids(&value), vec!["b", "c"]);
        let lone = FinishedTable { winners: vec![], others: vec!["b".into()], ..finished };
        assert!(table("lobby", &lone, &guests).is_none());
    }
}
