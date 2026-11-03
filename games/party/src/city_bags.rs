//! Loot bags. A vault spills one bag for a small crew and more for a big
//! one; each bag is on the ground or carried, and every bag scores on its
//! own when it reaches the van or the police get it.
use super::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Bag {
    /// Where it lies when nobody carries it.
    pub at: (f32, f32),
    pub carrier: Option<u8>,
}

impl CityGame {
    pub(super) fn carrying(&self, seat: u8) -> bool {
        self.bags.iter().any(|b| b.carrier == Some(seat))
    }
    pub(super) fn any_carrier(&self) -> bool {
        self.bags.iter().any(|b| b.carrier.is_some())
    }
    pub(super) fn carrier_seats(&self) -> Vec<u8> {
        self.bags.iter().filter_map(|b| b.carrier).collect()
    }
    pub(super) fn loose_bags(&self) -> Vec<(f32, f32)> {
        self.bags
            .iter()
            .filter(|b| b.carrier.is_none())
            .map(|b| b.at)
            .collect()
    }
    /// The carried bag falls where its carrier stands.
    pub(super) fn drop_bag(&mut self, seat: u8, at: (f32, f32)) -> bool {
        if let Some(b) = self.bags.iter_mut().find(|b| b.carrier == Some(seat)) {
            b.carrier = None;
            b.at = at;
            return true;
        }
        false
    }
    /// Picks up the loose bag nearest `at` within `range`, same room.
    pub(super) fn take_bag(&mut self, seat: u8, at: (f32, f32), range: f32) -> bool {
        if self.carrying(seat) {
            return false;
        }
        let best = self
            .bags
            .iter_mut()
            .filter(|b| b.carrier.is_none() && reach(at, b.at, range))
            .min_by(|a, b| distance(a.at, at).total_cmp(&distance(b.at, at)));
        if let Some(b) = best {
            b.carrier = Some(seat);
            return true;
        }
        false
    }
    /// A bag is out of play: delivered by the crew, or taken by police.
    /// The job ends once no bags are left.
    pub(super) fn bag_scored(
        &mut self,
        carrier: Option<u8>,
        near: (f32, f32),
        side: Side,
        message: &'static str,
    ) {
        let index = match carrier {
            Some(seat) => self.bags.iter().position(|b| b.carrier == Some(seat)),
            None => self
                .bags
                .iter()
                .enumerate()
                .filter(|(_, b)| b.carrier.is_none())
                .min_by(|a, b| distance(a.1.at, near).total_cmp(&distance(b.1.at, near)))
                .map(|(i, _)| i),
        };
        let Some(index) = index else { return };
        self.bags.remove(index);
        self.night_score[if side == Side::Police { 0 } else { 1 }] += 1;
        if let Some(job) = self.jobs.last_mut() {
            if side == Side::Crew {
                job.bags_delivered += 1;
                job.reach(stats::Phase::Delivered);
            } else {
                job.bags_recovered += 1;
            }
        }
        if self.bags.is_empty() {
            self.end_job(message);
        }
    }
    /// The vault opens and spills its bags on the vault floor.
    pub(super) fn spill_bags(&mut self) {
        let count = self.config.bags.unwrap_or(self.tuning.bags);
        self.bags = (0..count)
            .map(|i| Bag {
                at: (BANK.0 - 0.6 + i as f32 * 0.9, BANK.1 - 1.4),
                carrier: None,
            })
            .collect();
    }
}
