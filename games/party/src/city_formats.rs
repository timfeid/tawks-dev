//! Match formats: one officer against one to four robbers, or two against
//! two to six. Each format scales the job, the checks and the police kit so
//! a matched pair of sides wins about half the time.
use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Tuning {
    /// Big crews must also hold the security desk while the power is cut.
    pub second_post: bool,
    /// How long a cut power line (and the desk) stays down.
    pub power_seconds: f32,
    /// Scales check timers and windows: above 1 is harder.
    pub check_difficulty: f32,
    /// Numbers in the vault combination.
    pub dial_numbers: u8,
    /// Bags the vault spills.
    pub bags: usize,
    /// Seconds between an officer's radio pings.
    pub ping_cooldown: f32,
    /// Officers' running speed on foot.
    pub police_speed: f32,
    /// How long an arrested robber sits in the cell.
    pub jail_seconds: f32,
    /// How fast a robber runs with a bag.
    pub carry_speed: f32,
    /// A bot officer checks the bank and the power station on this cycle,
    /// part of it on foot; 0 keeps them on the street.
    pub inspect_period: f32,
    /// Seconds before bot officers begin checking the bank and power posts.
    pub inspect_delay: f32,
}

/// The formats a city night supports, as (police, crew).
pub const FORMATS: [(u8, u8); 9] = [
    (1, 1),
    (1, 2),
    (1, 3),
    (1, 4),
    (2, 2),
    (2, 3),
    (2, 4),
    (2, 5),
    (2, 6),
];

/// How many officers a lobby of this size gets by default.
pub fn police_for(players: usize) -> u8 {
    if players >= 6 { 2 } else { 1 }
}

/// Whether the host may choose this many officers for this many players.
pub fn allowed(police: u8, players: usize) -> bool {
    let crew = players.saturating_sub(usize::from(police));
    match police {
        1 => (1..=4).contains(&crew),
        2 => (2..=6).contains(&crew),
        _ => false,
    }
}

impl Tuning {
    pub fn for_game(seats: usize, police: u8) -> Self {
        let police = police.max(1);
        let crew = seats.saturating_sub(usize::from(police)).max(1) as u8;
        Self::for_format(police, crew)
    }
    /// Tuned with the seeded balance sweep (tests/city_balance.rs) so
    /// matched bots win about half their nights on either side.
    pub fn for_format(police: u8, crew: u8) -> Self {
        // (power, difficulty, police speed, jail, ping, carry speed, inspect period)
        let (
            power_seconds,
            check_difficulty,
            police_speed,
            jail_seconds,
            ping_cooldown,
            carry_speed,
            inspect_period,
        ) = match (police.clamp(1, 2), crew) {
            (1, 0..=1) => (22., 1.2, 6.65, 30., 30., 5.4, 0.),
            (1, 2) => (23., 1.4, 7.2, 30., 20., 4.8, 80.),
            (1, 3) => (28., 1.25, 6.9, 28., 20., 4.4, 0.),
            (1, _) => (45., 0.75, 7.9, 38., 15., 4.8, 30.),
            (_, 0..=2) => (32., 1.0, 5.5, 22., 35., 5.4, 0.),
            (_, 3) => (35., 0.95, 5.9, 24., 30., 5.4, 0.),
            (_, 4) => (30., 1.1, 5.9, 26., 25., 5.4, 0.),
            (_, 5) => (35., 1.0, 6.0, 26., 25., 5.4, 0.),
            (_, _) => (35., 0.95, 6.0, 26., 20., 5.2, 0.),
        };
        Tuning {
            second_post: crew >= 4,
            power_seconds,
            check_difficulty,
            dial_numbers: if crew >= 5 { 4 } else { 3 },
            bags: match crew {
                0..=3 => 1,
                4..=5 => 2,
                _ => 3,
            },
            ping_cooldown,
            police_speed,
            jail_seconds,
            carry_speed,
            inspect_period,
            inspect_delay: if police == 1 && crew == 2 { 240. } else { 0. },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn lobby_sizes_pick_a_supported_format() {
        for players in 2..=8 {
            let police = police_for(players);
            assert!(allowed(police, players), "{players} players");
        }
        assert!(!allowed(1, 7), "1v6 is not a format");
        assert!(!allowed(2, 3), "2v1 is not a format");
        assert!(
            allowed(2, 4) && allowed(1, 4),
            "four players may play 1v3 or 2v2"
        );
    }
    #[test]
    fn bigger_crews_hold_a_second_post_and_split_the_loot() {
        assert!(!Tuning::for_format(1, 3).second_post);
        assert!(Tuning::for_format(1, 4).second_post);
        assert_eq!(Tuning::for_format(2, 6).bags, 3);
    }
}
