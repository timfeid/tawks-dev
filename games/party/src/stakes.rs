//! Tells Tawks when a lobby game starts, finishes or is abandoned, so token
//! stakes and predictions on it can lock, pay out or refund.

use serde_json::{json, Value};

use crate::room::{Phase, Room};

struct Running {
    game_key: String,
    bout_at_start: Option<u32>,
}

#[derive(Default)]
pub struct StakeTracker {
    running: Option<Running>,
    /// A game just finished; nothing new starts until the room is back in its lobby.
    wait_for_lobby: bool,
}

fn report(
    room: &Room,
    game_key: &str,
    event: &str,
    players: Vec<String>,
    places: Vec<(String, u32)>,
) -> Value {
    json!({
        "lobby_id": room.match_id,
        "game_key": game_key,
        "event": event,
        "players": players,
        "places": places.into_iter().map(|(user_id, place)| json!({ "user_id": user_id, "place": place })).collect::<Vec<_>>(),
    })
}

fn seat_places(room: &Room, standings: &[Vec<u8>]) -> Vec<(String, u32)> {
    room.members
        .iter()
        .filter(|m| !m.is_bot)
        .filter_map(|m| {
            standings
                .iter()
                .position(|group| group.contains(&m.seat))
                .map(|i| (m.user_id.clone(), i as u32 + 1))
        })
        .collect()
}

impl StakeTracker {
    /// `game_stake_report` arguments for whatever changed since the last call.
    pub fn observe(&mut self, room: &Room) -> Vec<Value> {
        let mut out = Vec::new();
        if room.sandbox {
            self.running = None;
            return out;
        }
        let live = room.phase != Phase::Lobby;
        if self.wait_for_lobby {
            self.wait_for_lobby = live;
            return out;
        }
        let bout = room.last_bout.as_ref().map(|b| b.number);
        match self.running.as_ref() {
            None if live => {
                let game_key = if room.board.is_some() {
                    "tawkathon".to_string()
                } else {
                    room.minigame.to_string()
                };
                let players = room
                    .members
                    .iter()
                    .filter(|m| m.in_round && !m.is_bot)
                    .map(|m| m.user_id.clone())
                    .collect();
                out.push(report(room, &game_key, "started", players, Vec::new()));
                self.running = Some(Running {
                    game_key,
                    bout_at_start: bout,
                });
            }
            Some(running) if !live => {
                out.push(report(
                    room,
                    &running.game_key,
                    "abandoned",
                    Vec::new(),
                    Vec::new(),
                ));
                self.running = None;
            }
            Some(running) => {
                let finished = if let Phase::GameOver { standings, .. } = &room.phase {
                    Some(seat_places(room, standings))
                } else if room.board.is_none()
                    && room.table.is_none()
                    && bout != running.bout_at_start
                {
                    room.last_bout
                        .as_ref()
                        .filter(|played| played.round == 0)
                        .map(|played| {
                            played
                                .players
                                .iter()
                                .map(|(user, place)| (user.clone(), *place as u32))
                                .collect()
                        })
                } else {
                    None
                };
                if let Some(places) = finished {
                    out.push(report(
                        room,
                        &running.game_key,
                        "finished",
                        Vec::new(),
                        places,
                    ));
                    self.running = None;
                    self.wait_for_lobby = true;
                }
            }
            None => {}
        }
        out
    }

    /// The room is closing: a game still running ended abnormally.
    pub fn closed(&mut self, room: &Room) -> Option<Value> {
        let running = self.running.take()?;
        Some(report(
            room,
            &running.game_key,
            "abandoned",
            Vec::new(),
            Vec::new(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::PlayedBout;

    #[test]
    fn a_lone_bout_starts_finishes_and_waits_for_the_lobby() {
        let mut room = Room::new("lobby-1", 1);
        let mut tracker = StakeTracker::default();
        assert!(tracker.observe(&room).is_empty());
        room.phase = Phase::Playing;
        let started = tracker.observe(&room);
        assert_eq!(started.len(), 1);
        assert_eq!(started[0]["event"], "started");
        assert_eq!(started[0]["lobby_id"], "lobby-1");
        assert!(tracker.observe(&room).is_empty());
        room.last_bout = Some(PlayedBout {
            number: 1,
            key: "sumo",
            round: 0,
            players: vec![("a".into(), 1), ("b".into(), 2)],
        });
        let finished = tracker.observe(&room);
        assert_eq!(finished[0]["event"], "finished");
        assert_eq!(finished[0]["places"][1]["place"], 2);
        assert!(
            tracker.observe(&room).is_empty(),
            "results showing is not a new game"
        );
        room.phase = Phase::Lobby;
        assert!(tracker.observe(&room).is_empty());
        room.phase = Phase::Playing;
        assert_eq!(tracker.observe(&room)[0]["event"], "started");
    }

    #[test]
    fn leaving_a_game_early_or_closing_the_room_abandons_it() {
        let mut room = Room::new("lobby-2", 1);
        let mut tracker = StakeTracker::default();
        room.phase = Phase::Playing;
        tracker.observe(&room);
        room.phase = Phase::Lobby;
        assert_eq!(tracker.observe(&room)[0]["event"], "abandoned");
        room.phase = Phase::Playing;
        tracker.observe(&room);
        assert_eq!(tracker.closed(&room).unwrap()["event"], "abandoned");
        assert!(tracker.closed(&room).is_none());
    }
}
