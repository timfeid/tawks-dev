//! The party cast. Every minigame and the future board share these
//! characters; the client builds each one from this description.

use serde::Serialize;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    Cap,
    Leaf,
    Antenna,
    LongEars,
    RoundEars,
    Horns,
    Beanie,
    Visor,
}

#[derive(Clone, Copy, Debug, Serialize)]
pub struct Character {
    pub id: &'static str,
    pub name: &'static str,
    pub body: &'static str,
    pub accent: &'static str,
    pub belly: &'static str,
    pub feature: Feature,
}

pub const ROSTER: &[Character] = &[
    Character {
        id: "sunny",
        name: "Sunny",
        body: "#ffd23f",
        accent: "#ff7b29",
        belly: "#fff3c4",
        feature: Feature::Cap,
    },
    Character {
        id: "clover",
        name: "Clover",
        body: "#62c370",
        accent: "#2d7f45",
        belly: "#dff5d8",
        feature: Feature::Leaf,
    },
    Character {
        id: "blip",
        name: "Blip",
        body: "#4d96ff",
        accent: "#1d4ea3",
        belly: "#d6e6ff",
        feature: Feature::Antenna,
    },
    Character {
        id: "rosie",
        name: "Rosie",
        body: "#ff6f9f",
        accent: "#c2185b",
        belly: "#ffe0ea",
        feature: Feature::LongEars,
    },
    Character {
        id: "bruno",
        name: "Bruno",
        body: "#b07a4f",
        accent: "#6b4226",
        belly: "#f1dcc3",
        feature: Feature::RoundEars,
    },
    Character {
        id: "vex",
        name: "Vex",
        body: "#9b5de5",
        accent: "#4f1d8a",
        belly: "#eadcff",
        feature: Feature::Horns,
    },
    Character {
        id: "frost",
        name: "Frost",
        body: "#e9f3fa",
        accent: "#3f9bd8",
        belly: "#ffffff",
        feature: Feature::Beanie,
    },
    Character {
        id: "bolt",
        name: "Bolt",
        body: "#ff5e5b",
        accent: "#2b2d42",
        belly: "#ffe1d6",
        feature: Feature::Visor,
    },
];

pub fn character(id: &str) -> Option<&'static Character> {
    ROSTER.iter().find(|c| c.id == id)
}
