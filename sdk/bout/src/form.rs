//! Good days and bad days for bots. A bot's skill is its average; each bout
//! it arrives hot or cold, and each round it can run a little hot or cold
//! again or have an off round outright. The spread is symmetric, so bots of
//! equal skill stay even, and it widens with skill, so the best bot at a
//! mixed table is beatable without skill ceasing to matter on average.

use crate::Contestant;

/// Spread, in skill, of the whole-bout hot or cold swing at skill 0 and 1.
const BOUT_SPREAD: (f32, f32) = (0.10, 0.20);
/// Spread, in skill, of the per-round swing at skill 0 and 1.
const ROUND_SPREAD: (f32, f32) = (0.05, 0.10);
/// Chance of an off round at skill 0 and 1.
const OFF_ROUND: (f32, f32) = (0.08, 0.14);
/// What an off round leaves of a bot's skill.
const OFF_ROUND_KEEPS: f32 = 0.55;

fn lerp((low, high): (f32, f32), t: f32) -> f32 {
    low + (high - low) * t
}

fn mix(mut x: u64) -> u64 {
    x = x.wrapping_add(0x9e37_79b9_7f4a_7c15);
    x = (x ^ (x >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    x ^ (x >> 31)
}

struct Draws(u64);

impl Draws {
    fn new(seed: u64, seat: u8, round: u32) -> Self {
        Draws(mix(seed ^ mix(u64::from(seat) << 32 | u64::from(round))))
    }
    fn unit(&mut self) -> f32 {
        self.0 = mix(self.0);
        (self.0 >> 40) as f32 / (1u64 << 24) as f32
    }
    /// Roughly normal, mean 0 and spread 1, bounded at about plus or minus 1.7.
    fn bell(&mut self) -> f32 {
        (self.unit() + self.unit() + self.unit() - 1.5) * 2.0
    }
}

/// A bot's skill for this whole bout: its usual level, run hot or cold.
pub fn bout_form(skill: f32, seed: u64, seat: u8) -> f32 {
    bout_form_scaled(skill, seed, seat, 1.0)
}

pub fn bout_form_scaled(skill: f32, seed: u64, seat: u8, scale: f32) -> f32 {
    let level = skill.clamp(0.0, 1.0);
    let mut draws = Draws::new(seed, seat, 0);
    (level + lerp(BOUT_SPREAD, level) * scale.max(0.0) * draws.bell()).clamp(0.0, 1.0)
}

/// A bot's skill for one round of a bout, from its form for the bout: a
/// little hotter or colder again, or now and then an off round.
pub fn round_form(skill: f32, seed: u64, seat: u8, round: u32) -> f32 {
    let level = skill.clamp(0.0, 1.0);
    let mut draws = Draws::new(seed, seat, round.saturating_add(1));
    let mut formed = level + lerp(ROUND_SPREAD, level) * draws.bell();
    if draws.unit() < lerp(OFF_ROUND, level) {
        formed *= OFF_ROUND_KEEPS;
    }
    formed.clamp(0.0, 1.0)
}

/// The table as it plays this bout: every bot on its form for the day.
/// People are left as they are.
pub fn on_form(contestants: &[Contestant], seed: u64) -> Vec<Contestant> {
    on_form_scaled(contestants, seed, 1.0)
}

pub fn on_form_scaled(contestants: &[Contestant], seed: u64, scale: f32) -> Vec<Contestant> {
    contestants
        .iter()
        .map(|c| Contestant {
            skill: if c.is_bot {
                bout_form_scaled(c.skill, seed, c.seat, scale)
            } else {
                c.skill
            },
            ..c.clone()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean(values: impl Iterator<Item = f32>) -> f32 {
        let all: Vec<f32> = values.collect();
        all.iter().sum::<f32>() / all.len() as f32
    }

    #[test]
    fn form_is_deterministic_and_stays_in_range() {
        for seed in 0..500 {
            for seat in 0..8 {
                let a = bout_form(0.7, seed, seat);
                assert_eq!(a, bout_form(0.7, seed, seat));
                assert!((0.0..=1.0).contains(&a));
                let r = round_form(a, seed, seat, 3);
                assert!((0.0..=1.0).contains(&r));
            }
        }
    }

    #[test]
    fn form_centres_on_the_usual_level_and_keeps_the_order() {
        let at = |skill: f32| mean((0..4000).map(|seed| bout_form(skill, seed, 1)));
        let steady = at(0.4);
        assert!((steady - 0.4).abs() < 0.02, "steady averages {steady}");
        assert!(at(0.15) < at(0.4) && at(0.4) < at(0.7) && at(0.7) < at(1.0));
    }

    #[test]
    fn the_better_bot_sometimes_has_the_worse_day() {
        let upsets = (0..2000)
            .filter(|&seed| {
                round_form(bout_form(1.0, seed, 0), seed, 0, 1)
                    < round_form(bout_form(0.7, seed, 1), seed, 1, 1)
            })
            .count();
        assert!((100..900).contains(&upsets), "{upsets} upsets in 2000");
    }

    #[test]
    fn people_keep_their_skill() {
        let table = [Contestant::person(0), Contestant::bot(1, 0.7)];
        let formed = on_form(&table, 9);
        assert_eq!(formed[0].skill, 1.0);
        assert_eq!(formed[1].skill, bout_form(0.7, 9, 1));
        let unvaried = on_form_scaled(&table, 9, 0.0);
        assert_eq!(unvaried[0].skill, 1.0);
        assert_eq!(unvaried[1].skill, 0.7);
    }
}
