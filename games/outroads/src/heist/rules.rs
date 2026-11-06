//! What a run is played for: the cars, colours and values, the towns a
//! contract can name, and the lobby's settings.

use rand::Rng;
use serde::{Deserialize, Serialize};

use super::geo::LatLon;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Class {
    Sedan,
    Suv,
    Hatch,
    Pickup,
    Van,
    Cruiser,
}

impl Class {
    pub const PARKED: [Class; 4] = [Class::Sedan, Class::Suv, Class::Hatch, Class::Pickup];

    pub fn label(self) -> &'static str {
        match self {
            Class::Sedan => "sedan",
            Class::Suv => "SUV",
            Class::Hatch => "hatchback",
            Class::Pickup => "pickup",
            Class::Van => "van",
            Class::Cruiser => "cruiser",
        }
    }

    /// What the buyer pays for one in perfect shape, in-game cash.
    pub fn value(self) -> i64 {
        match self {
            Class::Sedan => 9_000,
            Class::Suv => 14_000,
            Class::Hatch => 7_000,
            Class::Pickup => 12_000,
            Class::Van | Class::Cruiser => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Colour {
    Black,
    Navy,
    Green,
    Grey,
    White,
    Silver,
    Red,
}

impl Colour {
    pub const ALL: [Colour; 7] = [
        Colour::Black,
        Colour::Navy,
        Colour::Green,
        Colour::Grey,
        Colour::White,
        Colour::Silver,
        Colour::Red,
    ];

    pub fn group(self) -> Shade {
        match self {
            Colour::Black | Colour::Navy | Colour::Green | Colour::Grey => Shade::Dark,
            Colour::White | Colour::Silver => Shade::Light,
            Colour::Red => Shade::Red,
        }
    }

    pub fn hex(self) -> &'static str {
        match self {
            Colour::Black => "#1d1f24",
            Colour::Navy => "#1f2f57",
            Colour::Green => "#22402e",
            Colour::Grey => "#4b4f56",
            Colour::White => "#e6e6e2",
            Colour::Silver => "#aeb3b8",
            Colour::Red => "#9c1d1d",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Shade {
    Dark,
    Light,
    Red,
}

impl Shade {
    pub fn label(self) -> &'static str {
        match self {
            Shade::Dark => "dark",
            Shade::Light => "light",
            Shade::Red => "red",
        }
    }
}

/// How well a car is looked after: better kept cars pay more and fight back harder.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Tier(pub u8);

impl Tier {
    pub fn unlocked_chance(self) -> f64 {
        [0.2, 0.1, 0.04][self.0.clamp(1, 3) as usize - 1]
    }
    pub fn alarm_chance(self) -> f64 {
        [0.35, 0.55, 0.8][self.0.clamp(1, 3) as usize - 1]
    }
    pub fn spare_key_chance(self) -> f64 {
        [0.06, 0.03, 0.01][self.0.clamp(1, 3) as usize - 1]
    }
    /// Half the lock ring's sweet spot, radians.
    pub fn lock_window(self) -> f64 {
        [0.42, 0.32, 0.24][self.0.clamp(1, 3) as usize - 1]
    }
    /// Seconds of being in the band it takes to start.
    pub fn ignition_time(self) -> f64 {
        [5.0, 7.0, 9.5][self.0.clamp(1, 3) as usize - 1]
    }
    pub fn band(self) -> f64 {
        [0.16, 0.12, 0.09][self.0.clamp(1, 3) as usize - 1]
    }
    pub fn value_factor(self) -> f64 {
        [1.0, 1.35, 1.8][self.0.clamp(1, 3) as usize - 1]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct Town {
    pub name: &'static str,
    pub county: &'static str,
    pub centre: LatLonS,
    pub radius: f64,
    /// More cars left unlocked here, as a gameplay knob.
    pub unlocked_bonus: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize)]
pub struct LatLonS {
    pub lat: f64,
    pub lon: f64,
}

impl From<LatLonS> for LatLon {
    fn from(p: LatLonS) -> LatLon {
        LatLon { lat: p.lat, lon: p.lon }
    }
}

const fn town(name: &'static str, county: &'static str, lat: f64, lon: f64, radius: f64, unlocked_bonus: f64) -> Town {
    Town {
        name,
        county,
        centre: LatLonS { lat, lon },
        radius,
        unlocked_bonus,
    }
}

/// Residential towns a drive from the port. Names only; nothing is tied to an address.
pub const TOWNS: &[Town] = &[
    town("Kearny", "Hudson", 40.7685, -74.1450, 900.0, 0.04),
    town("Harrison", "Hudson", 40.7465, -74.1560, 600.0, 0.02),
    town("Bayonne", "Hudson", 40.6688, -74.1146, 900.0, 0.03),
    town("Elizabeth", "Union", 40.6660, -74.2260, 900.0, 0.0),
    town("Union", "Union", 40.6985, -74.2610, 900.0, 0.05),
    town("Hillside", "Union", 40.6960, -74.2300, 700.0, 0.02),
    town("Belleville", "Essex", 40.7930, -74.1600, 800.0, 0.03),
    town("Nutley", "Essex", 40.8220, -74.1590, 900.0, 0.06),
    town("Bloomfield", "Essex", 40.8070, -74.1890, 900.0, 0.04),
];

/// Where crews start and deliver.
pub const PORT: LatLonS = LatLonS {
    lat: 40.6875,
    lon: -74.1500,
};
/// The gate zone, in meters east and south of the port origin.
pub const GATE: (f64, f64) = (0.0, 0.0);
pub const GATE_RADIUS: f64 = 22.0;
/// Precincts cops start from.
pub const PRECINCTS: &[LatLonS] = &[
    LatLonS {
        lat: 40.7350,
        lon: -74.1720,
    },
    LatLonS {
        lat: 40.6990,
        lon: -74.2200,
    },
    LatLonS {
        lat: 40.7600,
        lon: -74.1600,
    },
];

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Contract {
    pub class: Class,
    pub shade: Shade,
    pub towns: Vec<usize>,
}

impl Contract {
    pub fn pick(rng: &mut impl Rng) -> Contract {
        let class = Class::PARKED[rng.random_range(0..Class::PARKED.len())];
        let shade = [Shade::Dark, Shade::Dark, Shade::Light, Shade::Red][rng.random_range(0..4)];
        let first = rng.random_range(0..TOWNS.len());
        let mut second = rng.random_range(0..TOWNS.len());
        while second == first {
            second = rng.random_range(0..TOWNS.len());
        }
        Contract {
            class,
            shade,
            towns: vec![first, second],
        }
    }

    pub fn matches(&self, class: Class, colour: Colour) -> bool {
        class == self.class && colour.group() == self.shade
    }

    pub fn describe(&self) -> String {
        let towns: Vec<&str> = self.towns.iter().map(|&t| TOWNS[t].name).collect();
        format!("{} {}, {}", self.shade.label(), self.class.label(), towns.join(" or "))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Difficulty {
    Easy,
    Normal,
    Hard,
}

impl Difficulty {
    pub fn ai_cops(self) -> usize {
        match self {
            Difficulty::Easy => 2,
            Difficulty::Normal => 4,
            Difficulty::Hard => 6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    pub max_thieves: u8,
    pub max_cops: u8,
    pub difficulty: Difficulty,
    pub minutes: u8,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            max_thieves: 4,
            max_cops: 6,
            difficulty: Difficulty::Normal,
            minutes: 15,
        }
    }
}

impl Settings {
    pub fn checked(self) -> Settings {
        Settings {
            max_thieves: self.max_thieves.clamp(1, 4),
            max_cops: self.max_cops.min(6),
            difficulty: self.difficulty,
            minutes: self.minutes.clamp(10, 20),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    #[test]
    fn contracts_name_two_towns_and_match_by_class_and_shade() {
        let mut rng = rand::rngs::StdRng::seed_from_u64(3);
        for _ in 0..50 {
            let c = Contract::pick(&mut rng);
            assert_ne!(c.towns[0], c.towns[1]);
            let colour = Colour::ALL.iter().copied().find(|col| col.group() == c.shade).unwrap();
            assert!(c.matches(c.class, colour));
            assert!(!c.matches(Class::Van, colour));
        }
        let c = Contract {
            class: Class::Suv,
            shade: Shade::Dark,
            towns: vec![0, 4],
        };
        assert_eq!(c.describe(), "dark SUV, Kearny or Union");
    }

    #[test]
    fn settings_stay_in_range() {
        let s = Settings {
            max_thieves: 9,
            max_cops: 20,
            difficulty: Difficulty::Hard,
            minutes: 3,
        }
        .checked();
        assert_eq!((s.max_thieves, s.max_cops, s.minutes), (4, 6, 10));
    }
}
