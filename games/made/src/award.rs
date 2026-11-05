//! Turns a finished round into the report Tawks pays from.
use made_protocol::RosterSeat;
use made_protocol::protocol::EndReason;
use serde_json::{Value, json};

pub const GAME: &str = "made";

/// Only a round that played out pays; a crash pays nobody.
pub fn pays(reason: &EndReason) -> bool {
    !matches!(reason, EndReason::Crashed)
}

/// People by place, ties sharing one. Bot seats keep their places, so a
/// person behind one is not moved up, but they are not reported. People
/// seated but left out of the placements played without a place.
pub fn players(roster: &[RosterSeat], placements: &[Vec<u8>]) -> Vec<Value> {
    let person = |seat: u8| {
        roster
            .iter()
            .find(|s| s.seat == seat && !s.bot)
            .and_then(|s| s.user_id.clone())
    };
    let mut out: Vec<Value> = Vec::new();
    let mut seen: Vec<String> = Vec::new();
    let mut place = 1usize;
    for group in placements {
        for seat in group {
            if let Some(user) = person(*seat).filter(|u| !seen.contains(u)) {
                out.push(json!({ "user_id": user, "place": place }));
                seen.push(user);
            }
        }
        place += group.len().max(1);
    }
    for seat in roster {
        if let Some(user) = person(seat.seat).filter(|u| !seen.contains(u)) {
            out.push(json!({ "user_id": user }));
            seen.push(user);
        }
    }
    out
}

/// `room_opened_ms` tells apart rooms the relay opened for the same lobby at
/// different times, since each one counts its rounds from one.
pub fn reference_id(lobby_id: &str, room_opened_ms: u64, round: u32) -> String {
    format!("{GAME}:{lobby_id}:{room_opened_ms}:{round}")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seat(seat: u8, user: Option<&str>) -> RosterSeat {
        RosterSeat {
            seat,
            name: format!("Seat {seat}"),
            bot: user.is_none(),
            skill: None,
            user_id: user.map(str::to_string),
            avatar_url: None,
            costume: None,
            connected: true,
        }
    }

    #[test]
    fn places_follow_seats_and_bots_keep_theirs() {
        let roster = [seat(1, Some("ann")), seat(2, None), seat(3, Some("bob")), seat(4, Some("cy"))];
        let out = players(&roster, &[vec![2], vec![1, 3]]);
        assert_eq!(
            out,
            vec![
                json!({ "user_id": "ann", "place": 2 }),
                json!({ "user_id": "bob", "place": 2 }),
                json!({ "user_id": "cy" }),
            ]
        );
    }

    #[test]
    fn crashed_rounds_do_not_pay() {
        assert!(pays(&EndReason::Finished));
        assert!(pays(&EndReason::Timeout));
        assert!(!pays(&EndReason::Crashed));
    }

    #[test]
    fn rounds_and_reopened_rooms_get_new_references() {
        assert_ne!(reference_id("l1", 5, 1), reference_id("l1", 5, 2));
        assert_ne!(reference_id("l1", 5, 1), reference_id("l1", 6, 1));
    }
}
