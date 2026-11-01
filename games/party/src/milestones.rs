//! Milestone events for Tawkathons and bouts, read off the room as it plays.

use std::collections::{HashMap, HashSet};

use serde_json::{Value, json};

use crate::room::{Phase, PlayedBout, Room};

pub const GAME_ID: &str = "tawkathon";
/// Bots and an empty room are not an audience: results need two people.
const MIN_PEOPLE: usize = 2;
const PERFECT_MIN_BOUTS: u32 = 5;

#[derive(Default)]
struct Party {
    round: u32,
    bouts: u32,
    wins: HashMap<String, u32>,
    /// Seats that sat alone in last place after some round.
    trailed: HashSet<u8>,
    finished: bool,
}

#[derive(Default)]
pub struct MilestoneTracker {
    guests: HashSet<String>,
    last_bout: Option<(u32, u32, &'static str)>,
    party: Option<Party>,
}

impl MilestoneTracker {
    pub fn note_player(&mut self, user_id: &str, guest: bool) {
        if guest {
            self.guests.insert(user_id.to_string());
        } else {
            self.guests.remove(user_id);
        }
    }

    /// `milestone_event` arguments for whatever finished since the last call.
    pub fn observe(&mut self, room: &Room) -> Vec<Value> {
        let mut out = Vec::new();
        let bout = room.last_bout.as_ref().map(|b| (b.number, b.round, b.key));
        if room.sandbox {
            self.party = None;
            self.last_bout = bout;
            return out;
        }
        let board = room.board.as_ref().filter(|_| !room.city_mode);
        match (board, self.party.as_mut()) {
            (None, _) => self.party = None,
            (Some(board), None) => {
                self.party = Some(Party {
                    round: board.round,
                    ..Party::default()
                })
            }
            (Some(board), Some(party)) if board.round != party.round => {
                party.round = board.round;
                if let Some(seat) = board.handicapped() {
                    party.trailed.insert(seat);
                }
            }
            _ => {}
        }
        if bout.is_some() && bout != self.last_bout {
            if let Some(played) = room.last_bout.as_ref() {
                self.bout_finished(room, played, &mut out);
            }
        }
        self.last_bout = bout;
        if let (Phase::GameOver { standings, .. }, Some(party)) = (&room.phase, self.party.as_ref())
        {
            if !party.finished {
                self.party_finished(room, party, standings, &mut out);
            }
            if let Some(party) = self.party.as_mut() {
                party.finished = true;
            }
        }
        out
    }

    fn bout_finished(&mut self, room: &Room, played: &PlayedBout, out: &mut Vec<Value>) {
        if let Some(party) = self.party.as_mut().filter(|_| played.round > 0) {
            party.bouts += 1;
            for (user, _) in played.players.iter().filter(|(_, place)| *place == 1) {
                *party.wins.entry(user.clone()).or_default() += 1;
            }
        }
        let people = played.players.len();
        if people < MIN_PEOPLE {
            return;
        }
        let decided = played.players.iter().any(|(_, place)| *place > 1);
        for (user, place) in &played.players {
            if self.guests.contains(user) || *place == 0 {
                continue;
            }
            out.push(event(
                room,
                user,
                "bout_finished",
                json!({
                    "bout": played.key,
                    "won": decided && *place == 1,
                    "place": place,
                    "players": people,
                    "in_party": played.round > 0,
                }),
            ));
        }
    }

    fn party_finished(
        &self,
        room: &Room,
        party: &Party,
        standings: &[Vec<u8>],
        out: &mut Vec<Value>,
    ) {
        let seated: Vec<_> = room.members.iter().filter(|m| m.in_round).collect();
        let people = seated.iter().filter(|m| !m.is_bot).count();
        if people < MIN_PEOPLE {
            return;
        }
        for member in seated
            .iter()
            .filter(|m| !m.is_bot && !self.guests.contains(&m.user_id))
        {
            let Some(place) = standings
                .iter()
                .position(|group| group.contains(&member.seat))
                .map(|i| i + 1)
            else {
                continue;
            };
            let won = place == 1 && standings.len() > 1;
            let bouts_won = party.wins.get(&member.user_id).copied().unwrap_or(0);
            out.push(event(
                room,
                &member.user_id,
                "party_finished",
                json!({
                    "won": won,
                    "place": place,
                    "players": people,
                    "seats": seated.len(),
                    "bouts": party.bouts,
                    "bouts_won": bouts_won,
                    "perfect": won && party.bouts >= PERFECT_MIN_BOUTS && bouts_won == party.bouts,
                    "comeback": won && party.trailed.contains(&member.seat),
                }),
            ));
        }
    }
}

fn event(room: &Room, user_id: &str, event_key: &str, meta: Value) -> Value {
    json!({
        "game_id": GAME_ID,
        "event_key": event_key,
        "user_id": user_id,
        "lobby_id": room.match_id,
        "meta": meta,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::room::TICK_HZ;

    fn bout(number: u32, players: &[(&str, usize)]) -> Option<PlayedBout> {
        Some(PlayedBout {
            number,
            key: "sumo",
            round: 0,
            players: players
                .iter()
                .map(|(user, place)| (user.to_string(), *place))
                .collect(),
        })
    }

    #[test]
    fn a_bout_reports_each_signed_in_person_once() {
        let mut room = Room::new("lobby", 1);
        let mut tracker = MilestoneTracker::default();
        tracker.note_player("guest", true);
        assert!(tracker.observe(&room).is_empty());
        room.last_bout = bout(1, &[("a", 1), ("b", 2), ("guest", 3)]);
        let events = tracker.observe(&room);
        assert_eq!(events.len(), 2);
        assert_eq!(events[0]["user_id"], "a");
        assert_eq!(events[0]["event_key"], "bout_finished");
        assert_eq!(events[0]["lobby_id"], "lobby");
        assert_eq!(events[0]["meta"]["won"], true);
        assert_eq!(events[0]["meta"]["bout"], "sumo");
        assert_eq!(events[1]["meta"]["won"], false);
        assert!(
            tracker.observe(&room).is_empty(),
            "the same bout is not reported twice"
        );
        room.last_bout = bout(2, &[("a", 1), ("b", 1)]);
        let events = tracker.observe(&room);
        assert!(
            events.iter().all(|e| e["meta"]["won"] == false),
            "a tie for first among everyone is no win"
        );
        room.last_bout = bout(3, &[("a", 1)]);
        assert!(
            tracker.observe(&room).is_empty(),
            "a bout alone against bots earns nothing"
        );
    }

    #[test]
    fn a_finished_tawkathon_reports_once_per_person() {
        let mut room = Room::new("lobby", 7);
        room.connect("a", "A", "").unwrap();
        room.add_bot("a").unwrap();
        room.add_bot("a").unwrap();
        room.start_party("a").unwrap();
        room.board.as_mut().unwrap().rounds = 2;
        // Bots play every seat; the first two count as people when observed.
        for member in room.members.iter_mut() {
            member.is_bot = true;
        }
        let mut tracker = MilestoneTracker::default();
        let mut finished = Vec::new();
        let mut bouts = 0;
        let mut last = None;
        for _ in 0..TICK_HZ as u64 * 60 * 60 {
            room.step();
            let number = room.last_bout.as_ref().map(|b| b.number);
            if number.is_some() && number != last && room.board.is_some() {
                bouts += 1;
            }
            last = number;
            for member in room.members.iter_mut().take(2) {
                member.is_bot = false;
            }
            finished.extend(
                tracker
                    .observe(&room)
                    .into_iter()
                    .filter(|e| e["event_key"] == "party_finished"),
            );
            for member in room.members.iter_mut() {
                member.is_bot = true;
            }
            if room.phase == Phase::Lobby && !finished.is_empty() {
                break;
            }
        }
        assert_eq!(finished.len(), 2, "{finished:?}");
        for event in &finished {
            let meta = &event["meta"];
            assert_eq!(meta["players"], 2);
            assert_eq!(meta["seats"], 3);
            assert_eq!(meta["bouts"], bouts);
            assert_eq!(meta["won"], meta["place"] == 1);
        }
    }
}
