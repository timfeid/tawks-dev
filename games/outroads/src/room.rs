//! One lobby's drive: where everyone spawns, the places put to a vote, the
//! time of day, and who is driving in what colour. Car poses pass through
//! the server without being kept here beyond the latest one per driver.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::pass::Pass;

pub const COLORS: [&str; 10] = [
    "#e8553d", "#2f7de1", "#f2b632", "#36b37e", "#9b5de5", "#f15bb5", "#00b8d9", "#ff8b3d", "#8d9db6", "#e0e0d1",
];
pub const MAX_NOMINATIONS: usize = 6;
const MAX_NAME: usize = 120;
/// Cars stay within this many meters of the spawn point.
pub const WORLD_RADIUS_M: f64 = 60_000.0;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Place {
    pub name: String,
    pub lat: f64,
    pub lon: f64,
}

impl Place {
    pub fn checked(self) -> Result<Place, &'static str> {
        let name: String = self.name.trim().chars().take(MAX_NAME).collect();
        if name.is_empty() {
            return Err("That place has no name");
        }
        let lat_ok = self.lat.is_finite() && self.lat.abs() <= 84.0;
        let lon_ok = self.lon.is_finite() && self.lon.abs() <= 180.0;
        if !lat_ok || !lon_ok {
            return Err("That place is off the map");
        }
        Ok(Place {
            name,
            lat: (self.lat * 1e6).round() / 1e6,
            lon: (self.lon * 1e6).round() / 1e6,
        })
    }

    fn same_spot(&self, other: &Place) -> bool {
        (self.lat - other.lat).abs() < 1e-4 && (self.lon - other.lon).abs() < 1e-4
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Action {
    /// The host moves everyone there.
    SetPlace {
        place: Place,
    },
    /// Anyone puts a place to the vote.
    Nominate {
        place: Place,
    },
    Vote {
        id: u32,
    },
    Unvote,
    /// Hour of the day in the world, 0 to 24.
    SetHour {
        hour: f32,
    },
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub x: f32,
    pub z: f32,
    /// Heading in radians, 0 facing north, growing clockwise.
    pub h: f32,
    /// Speed in meters per second, signed.
    pub s: f32,
    /// Steering, -1 to 1.
    pub st: f32,
    /// Bit 1 headlights, bit 2 braking.
    pub f: u8,
}

impl Pose {
    pub fn checked(self) -> Option<Pose> {
        let finite = [self.x, self.z, self.h, self.s, self.st].iter().all(|v| v.is_finite());
        let inside = (self.x as f64).hypot(self.z as f64) <= WORLD_RADIUS_M;
        (finite && inside).then_some(Pose {
            s: self.s.clamp(-30.0, 90.0),
            st: self.st.clamp(-1.0, 1.0),
            h: self.h.rem_euclid(std::f32::consts::TAU),
            f: self.f & 0b11,
            ..self
        })
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Nomination {
    pub id: u32,
    pub place: Place,
    pub by: String,
    pub votes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Driver {
    pub user_id: String,
    pub name: String,
    pub avatar_url: String,
    pub color: &'static str,
    #[serde(skip)]
    pub owner: bool,
    #[serde(skip)]
    pub joined: u64,
}

#[derive(Debug, Default)]
pub struct Room {
    pub place: Option<Place>,
    /// Goes up every time the place changes, so clients know to respawn.
    pub epoch: u32,
    pub hour: f32,
    pub nominations: Vec<Nomination>,
    next_nomination: u32,
    drivers: BTreeMap<String, Driver>,
    joins: u64,
    colors_used: Vec<(String, usize)>,
}

#[derive(Debug, Serialize)]
pub struct RoomView<'a> {
    pub host: Option<&'a str>,
    pub place: Option<&'a Place>,
    pub epoch: u32,
    pub hour: f32,
    pub nominations: &'a [Nomination],
    pub drivers: Vec<&'a Driver>,
}

/// What an action changed, so the server knows what to send.
#[derive(Debug, PartialEq, Eq)]
pub enum Changed {
    Room,
    Moved,
}

impl Room {
    pub fn new() -> Room {
        Room {
            hour: 15.5,
            ..Room::default()
        }
    }

    /// Seats a driver, keeping the colour they had if they were here before.
    pub fn join(&mut self, pass: &Pass) -> &Driver {
        self.joins += 1;
        let color = self.color_for(&pass.user_id);
        let joined = self.joins;
        self.drivers
            .entry(pass.user_id.clone())
            .and_modify(|d| {
                d.name = pass.name.clone();
                d.avatar_url = pass.avatar_url.clone();
                d.owner |= pass.owner;
            })
            .or_insert_with(|| Driver {
                user_id: pass.user_id.clone(),
                name: pass.name.clone(),
                avatar_url: pass.avatar_url.clone(),
                color,
                owner: pass.owner,
                joined,
            })
    }

    pub fn leave(&mut self, user_id: &str) {
        self.drivers.remove(user_id);
        for n in &mut self.nominations {
            n.votes.retain(|v| v != user_id);
        }
    }

    pub fn is_empty(&self) -> bool {
        self.drivers.is_empty()
    }

    pub fn driver(&self, user_id: &str) -> Option<&Driver> {
        self.drivers.get(user_id)
    }

    fn color_for(&mut self, user_id: &str) -> &'static str {
        if let Some((_, i)) = self.colors_used.iter().find(|(u, _)| u == user_id) {
            return COLORS[*i];
        }
        let taken: Vec<usize> = self
            .drivers
            .values()
            .filter_map(|d| COLORS.iter().position(|c| *c == d.color))
            .collect();
        let i = (0..COLORS.len())
            .find(|i| !taken.contains(i))
            .unwrap_or(self.colors_used.len() % COLORS.len());
        self.colors_used.push((user_id.to_string(), i));
        COLORS[i]
    }

    /// The lobby owner while they are here, otherwise whoever has been here longest.
    pub fn host(&self) -> Option<&str> {
        self.drivers
            .values()
            .min_by_key(|d| (!d.owner, d.joined))
            .map(|d| d.user_id.as_str())
    }

    pub fn view(&self) -> RoomView<'_> {
        let mut drivers: Vec<&Driver> = self.drivers.values().collect();
        drivers.sort_by_key(|d| d.joined);
        RoomView {
            host: self.host(),
            place: self.place.as_ref(),
            epoch: self.epoch,
            hour: self.hour,
            nominations: &self.nominations,
            drivers,
        }
    }

    /// A place a reconnecting client remembers, taken only while the room has
    /// none, so a restarted server picks the drive back up where it was.
    pub fn resume(&mut self, place: Place) {
        if self.place.is_none()
            && let Ok(place) = place.checked()
        {
            self.place = Some(place);
            self.epoch += 1;
        }
    }

    pub fn act(&mut self, user_id: &str, action: Action) -> Result<Changed, &'static str> {
        if !self.drivers.contains_key(user_id) {
            return Err("Join the drive first");
        }
        match action {
            Action::SetPlace { place } => {
                if self.host() != Some(user_id) {
                    return Err("Only the host picks the place. Put it to a vote instead.");
                }
                self.move_to(place.checked()?);
                Ok(Changed::Moved)
            }
            Action::Nominate { place } => {
                let place = place.checked()?;
                let id = match self.nominations.iter().find(|n| n.place.same_spot(&place)) {
                    Some(n) => n.id,
                    None => {
                        if self.nominations.len() >= MAX_NOMINATIONS {
                            self.nominations.remove(0);
                        }
                        self.next_nomination += 1;
                        self.nominations.push(Nomination {
                            id: self.next_nomination,
                            place,
                            by: user_id.to_string(),
                            votes: Vec::new(),
                        });
                        self.next_nomination
                    }
                };
                self.vote(user_id, id)
            }
            Action::Vote { id } => self.vote(user_id, id),
            Action::Unvote => {
                for n in &mut self.nominations {
                    n.votes.retain(|v| v != user_id);
                }
                Ok(Changed::Room)
            }
            Action::SetHour { hour } => {
                if !hour.is_finite() {
                    return Err("That is not a time of day");
                }
                self.hour = hour.rem_euclid(24.0);
                Ok(Changed::Room)
            }
        }
    }

    fn vote(&mut self, user_id: &str, id: u32) -> Result<Changed, &'static str> {
        if !self.nominations.iter().any(|n| n.id == id) {
            return Err("That vote has closed");
        }
        for n in &mut self.nominations {
            n.votes.retain(|v| v != user_id);
            if n.id == id {
                n.votes.push(user_id.to_string());
            }
        }
        let online = self.drivers.len();
        let winner = self
            .nominations
            .iter()
            .find(|n| n.votes.len() * 2 > online)
            .map(|n| n.place.clone());
        match winner {
            Some(place) => {
                self.move_to(place);
                Ok(Changed::Moved)
            }
            None => Ok(Changed::Room),
        }
    }

    fn move_to(&mut self, place: Place) {
        self.place = Some(place);
        self.epoch += 1;
        self.nominations.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pass(user: &str, owner: bool) -> Pass {
        Pass {
            lobby_id: "l".into(),
            user_id: user.into(),
            name: user.into(),
            avatar_url: String::new(),
            owner,
            exp: i64::MAX,
        }
    }

    fn place(name: &str, lat: f64, lon: f64) -> Place {
        Place {
            name: name.into(),
            lat,
            lon,
        }
    }

    #[test]
    fn the_owner_hosts_while_here_otherwise_the_longest_seated() {
        let mut room = Room::new();
        room.join(&pass("ash", false));
        room.join(&pass("bea", false));
        assert_eq!(room.host(), Some("ash"));
        room.join(&pass("own", true));
        assert_eq!(room.host(), Some("own"));
        room.leave("own");
        room.leave("ash");
        assert_eq!(room.host(), Some("bea"));
    }

    #[test]
    fn drivers_get_distinct_colours_and_keep_theirs_on_rejoin() {
        let mut room = Room::new();
        let a = room.join(&pass("ash", false)).color;
        let b = room.join(&pass("bea", false)).color;
        assert_ne!(a, b);
        room.leave("ash");
        room.join(&pass("cal", false));
        assert_eq!(room.join(&pass("ash", false)).color, a);
    }

    #[test]
    fn only_the_host_sets_the_place_and_it_bumps_the_epoch() {
        let mut room = Room::new();
        room.join(&pass("ash", false));
        room.join(&pass("bea", false));
        let paris = place("Paris", 48.8566, 2.3522);
        assert!(room.act("bea", Action::SetPlace { place: paris.clone() }).is_err());
        assert_eq!(room.act("ash", Action::SetPlace { place: paris }), Ok(Changed::Moved));
        assert_eq!(room.epoch, 1);
        assert_eq!(room.place.as_ref().unwrap().name, "Paris");
    }

    #[test]
    fn a_majority_vote_moves_everyone() {
        let mut room = Room::new();
        for u in ["ash", "bea", "cal"] {
            room.join(&pass(u, false));
        }
        let tokyo = place("Shibuya", 35.6595, 139.7005);
        assert_eq!(
            room.act("bea", Action::Nominate { place: tokyo.clone() }),
            Ok(Changed::Room)
        );
        assert_eq!(room.nominations[0].votes, vec!["bea"]);
        assert_eq!(room.act("cal", Action::Nominate { place: tokyo }), Ok(Changed::Moved));
        assert_eq!(room.place.as_ref().unwrap().name, "Shibuya");
        assert!(room.nominations.is_empty());
    }

    #[test]
    fn each_driver_has_one_vote_and_leaving_takes_it_back() {
        let mut room = Room::new();
        for u in ["ash", "bea", "cal", "dee"] {
            room.join(&pass(u, false));
        }
        room.act(
            "ash",
            Action::Nominate {
                place: place("A", 1.0, 1.0),
            },
        )
        .unwrap();
        room.act(
            "bea",
            Action::Nominate {
                place: place("B", 2.0, 2.0),
            },
        )
        .unwrap();
        let a = room.nominations[0].id;
        room.act("bea", Action::Vote { id: a }).unwrap();
        assert_eq!(room.nominations[0].votes.len(), 2);
        assert!(room.nominations[1].votes.is_empty());
        room.leave("bea");
        assert_eq!(room.nominations[0].votes, vec!["ash"]);
        assert!(room.act("ash", Action::Vote { id: 99 }).is_err());
    }

    #[test]
    fn places_and_poses_are_checked() {
        assert!(place("", 1.0, 1.0).checked().is_err());
        assert!(place("Pole", 89.0, 0.0).checked().is_err());
        assert!(place("Nan", f64::NAN, 0.0).checked().is_err());
        let p = place("  Rome  ", 41.902_782_1, 12.496_366_2).checked().unwrap();
        assert_eq!((p.name.as_str(), p.lat), ("Rome", 41.902782));
        let pose = Pose {
            x: 10.0,
            z: -5.0,
            h: -0.5,
            s: 500.0,
            st: 3.0,
            f: 0xff,
        };
        let fixed = pose.checked().unwrap();
        assert!((fixed.h - (std::f32::consts::TAU - 0.5)).abs() < 1e-5);
        assert_eq!((fixed.s, fixed.st, fixed.f), (90.0, 1.0, 3));
        assert!(
            Pose {
                x: f32::INFINITY,
                ..pose
            }
            .checked()
            .is_none()
        );
        assert!(Pose { x: 70_000.0, ..pose }.checked().is_none());
    }

    #[test]
    fn a_remembered_place_only_fills_an_empty_room() {
        let mut room = Room::new();
        room.join(&pass("ash", false));
        room.resume(place("Old", 1.0, 1.0));
        assert_eq!(room.epoch, 1);
        room.resume(place("Other", 2.0, 2.0));
        assert_eq!(room.place.as_ref().unwrap().name, "Old");
    }
}
