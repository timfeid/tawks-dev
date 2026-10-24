use crate::types::ManaCost;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
pub struct ManaPool {
    pub neon: i32,
    pub glitch: i32,
    pub chrome: i32,
    pub slag: i32,
    pub bio: i32,
    pub colorless: i32,
}

impl ManaPool {
    pub fn can_afford(&self, cost: &ManaCost) -> bool {
        if self.neon < cost.neon {
            return false;
        }
        if self.glitch < cost.glitch {
            return false;
        }
        if self.chrome < cost.chrome {
            return false;
        }
        if self.slag < cost.slag {
            return false;
        }
        if self.bio < cost.bio {
            return false;
        }

        let leftover = (self.neon - cost.neon)
            + (self.glitch - cost.glitch)
            + (self.chrome - cost.chrome)
            + (self.slag - cost.slag)
            + (self.bio - cost.bio)
            + self.colorless;
        leftover >= cost.colorless
    }

    pub fn spend(&mut self, cost: &ManaCost) {
        self.neon -= cost.neon;
        self.glitch -= cost.glitch;
        self.chrome -= cost.chrome;
        self.slag -= cost.slag;
        self.bio -= cost.bio;
        // Basic colorless spending logic
        self.colorless -= cost.colorless;
    }

    pub fn spend_available(&mut self, cost: &mut ManaCost) {
        // 1. Spend colored mana for colored costs
        let neon_spend = self.neon.min(cost.neon);
        self.neon -= neon_spend;
        cost.neon -= neon_spend;

        let glitch_spend = self.glitch.min(cost.glitch);
        self.glitch -= glitch_spend;
        cost.glitch -= glitch_spend;

        let chrome_spend = self.chrome.min(cost.chrome);
        self.chrome -= chrome_spend;
        cost.chrome -= chrome_spend;

        let slag_spend = self.slag.min(cost.slag);
        self.slag -= slag_spend;
        cost.slag -= slag_spend;

        let bio_spend = self.bio.min(cost.bio);
        self.bio -= bio_spend;
        cost.bio -= bio_spend;

        // 2. Spend colorless pool for colorless costs
        let colorless_spend = self.colorless.min(cost.colorless);
        self.colorless -= colorless_spend;
        cost.colorless -= colorless_spend;

        // 3. Spend excess colored mana for remaining colorless costs
        if cost.colorless > 0 {
            let colors = [
                &mut self.neon,
                &mut self.glitch,
                &mut self.chrome,
                &mut self.slag,
                &mut self.bio,
            ];
            for pool_color in colors {
                if cost.colorless <= 0 {
                    break;
                }
                let spend = (*pool_color).min(cost.colorless);
                *pool_color -= spend;
                cost.colorless -= spend;
            }
        }
    }
}
