//! The party cast. Every minigame and the future board share these
//! characters; the client builds each one from this description.

use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

#[derive(Clone, Copy, Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Feature {
    None,
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
        name: "Yellow",
        body: "#ffd23f",
        accent: "#ff7b29",
        belly: "#fff3c4",
        feature: Feature::None,
    },
    Character {
        id: "clover",
        name: "Green",
        body: "#62c370",
        accent: "#2d7f45",
        belly: "#dff5d8",
        feature: Feature::None,
    },
    Character {
        id: "blip",
        name: "Blue",
        body: "#4d96ff",
        accent: "#1d4ea3",
        belly: "#d6e6ff",
        feature: Feature::None,
    },
    Character {
        id: "rosie",
        name: "Pink",
        body: "#ff6f9f",
        accent: "#c2185b",
        belly: "#ffe0ea",
        feature: Feature::None,
    },
    Character {
        id: "bruno",
        name: "Brown",
        body: "#b07a4f",
        accent: "#6b4226",
        belly: "#f1dcc3",
        feature: Feature::None,
    },
    Character {
        id: "vex",
        name: "Purple",
        body: "#9b5de5",
        accent: "#4f1d8a",
        belly: "#eadcff",
        feature: Feature::None,
    },
    Character {
        id: "frost",
        name: "White",
        body: "#e9f3fa",
        accent: "#3f9bd8",
        belly: "#ffffff",
        feature: Feature::None,
    },
    Character {
        id: "bolt",
        name: "Red",
        body: "#ff5e5b",
        accent: "#2b2d42",
        belly: "#ffe1d6",
        feature: Feature::None,
    },
    Character {
        id: "onyx",
        name: "Black",
        body: "#2b2b35",
        accent: "#9aa3b5",
        belly: "#4b4b58",
        feature: Feature::None,
    },
];

pub fn character(id: &str) -> Option<&'static Character> {
    ROSTER.iter().find(|c| c.id == id)
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Costume {
    pub color: String,
    #[serde(default, skip_serializing_if="Option::is_none")] pub body_width: Option<f32>,
    #[serde(default)] pub hat: Option<String>,
    #[serde(default)] pub shirt: Option<String>,
    #[serde(default)] pub pants: Option<String>,
    #[serde(default)] pub shoes: Option<String>,
    #[serde(default)] pub gloves: Option<String>,
    /// What items are carried in on the board, in place of the plain sack.
    #[serde(default)] pub bag: Option<String>,
    /// Any number worn together; older outfits saved one, as `extra`.
    #[serde(default, alias = "extra", deserialize_with = "one_or_many")] pub extras: Vec<String>,
    #[serde(default)] pub model_id: Option<String>,
    #[serde(default)] pub head: Option<String>,
    /// The worn PuckLab jersey's club design, as Tawks validated it. Passed through untouched.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub jersey: Option<serde_json::Value>,
    /// The face reaction made when celebrating, and with shuffle on, every one owned to pick from. Checked by Tawks.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub reaction: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")] pub shuffle_reactions: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")] pub reaction_pool: Vec<String>,
    /// The pet out beside the character and what it wears. Checked by Tawks.
    #[serde(default, skip_serializing_if = "PetLook::is_plain")] pub pet: PetLook,
    /// A palette colour per slot, in place of each item's own main colour. Checked by Tawks.
    #[serde(default, skip_serializing_if = "std::collections::BTreeMap::is_empty")] pub tints: std::collections::BTreeMap<String, String>,
}
pub const TINT_SLOTS: [&str; 7] = ["hat", "shirt", "pants", "shoes", "gloves", "bag", "extra"];
pub fn tint_palette() -> &'static [String] {
    #[derive(Deserialize)]
    struct Tint { color: String }
    static COLORS: OnceLock<Vec<String>> = OnceLock::new();
    COLORS.get_or_init(|| serde_json::from_str::<Vec<Tint>>(include_str!("../../canvases/src/party/costumes/tints.json")).expect("tint palette").into_iter().map(|tint| tint.color).collect())
}
/// Pet store item ids; `id` is an adopted pet shown in place of the hatched one.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct PetLook {
    #[serde(default, skip_serializing_if = "Option::is_none")] pub id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub coat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub hat: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub neck: Option<String>,
    /// What its owner calls it, and its growth stage. Filled by Tawks.
    #[serde(default, skip_serializing_if = "Option::is_none")] pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")] pub stage: Option<u8>,
}
impl PetLook {
    pub fn is_plain(&self) -> bool { *self == Self::default() }
    fn slots(&self) -> impl Iterator<Item = (&'static str, &str)> {
        [("pet", &self.id), ("pet-coat", &self.coat), ("pet-hat", &self.hat), ("pet-neck", &self.neck)].into_iter().filter_map(|(slot, id)| id.as_deref().map(|id| (slot, id)))
    }
}
/// A list, or the single id (or null) older outfits saved.
fn one_or_many<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum OneOrMany { One(Option<String>), Many(Vec<String>) }
    Ok(match OneOrMany::deserialize(deserializer)? { OneOrMany::One(one) => one.into_iter().collect(), OneOrMany::Many(many) => many })
}
#[derive(Clone, Debug, Deserialize)]
pub struct Wardrobe { pub costume: Costume, pub owned: Vec<String>, #[serde(default)] pub set_up: bool, #[serde(default)] pub jersey: Option<serde_json::Value> }
#[derive(Debug, Deserialize)]
pub struct Cosmetic {pub id:String,pub slot:String,pub name:String,pub price:i32,pub icon:String,#[serde(default)] pub included_with:Option<String>,#[serde(default)] pub unlocked_by:Option<String>}
impl Cosmetic {
    /// Sold in the store, rather than included with another product or earned from a milestone.
    pub fn for_sale(&self) -> bool { self.included_with.is_none() && self.unlocked_by.is_none() }
}
pub fn cosmetics() -> &'static [Cosmetic] {
    static ITEMS: OnceLock<Vec<Cosmetic>>=OnceLock::new();
    ITEMS.get_or_init(|| {
        let mut items: Vec<Cosmetic> = serde_json::from_str(include_str!("../../canvases/src/party/costumes/catalog.json")).expect("cosmetic catalog");
        let models: Vec<Cosmetic> = serde_json::from_str(include_str!("../../canvases/src/party/models/catalog.json")).expect("character model catalog");
        items.extend(models);
        items
    })
}

/// Face reactions for the character, sold through the same bot as clothing.
#[derive(Debug, Deserialize)]
pub struct Reaction {pub id:String,pub name:String,pub price:i32,pub icon:String,pub animation:String,pub description:String}
pub fn reactions() -> &'static [Reaction] {
    static ITEMS: OnceLock<Vec<Reaction>>=OnceLock::new();
    ITEMS.get_or_init(|| serde_json::from_str(include_str!("../../canvases/src/party/reactions/catalog.json")).expect("reaction catalog"))
}

/// Pets to adopt, and coats and gear for them, sold through the same bot as clothing.
#[derive(Debug, Deserialize)]
pub struct PetItem {pub id:String,pub slot:String,pub name:String,pub price:i32,pub icon:String,pub description:String}
pub fn pet_items() -> &'static [PetItem] {
    static ITEMS: OnceLock<Vec<PetItem>>=OnceLock::new();
    ITEMS.get_or_init(|| serde_json::from_str(include_str!("../../canvases/src/party/pets/store.json")).expect("pet store catalog"))
}

/// Comes with PuckLab, never sold; shows the owner's Lab club jersey.
pub const PUCKLAB_JERSEY: &str = "shirt-pucklab";

/// Only dev sandbox drafts bypass store ownership. Imported models still need
/// the account's saved, server-validated playable model, never a client claim.
pub fn validate_sandbox_costume(costume: &Costume, wardrobe: Option<&Wardrobe>) -> Result<(), &'static str> {
    if costume.body_width.is_some_and(|width| !width.is_finite() || !(0.7..=1.4).contains(&width)) { return Err("Choose a body build between slim and round."); }
    if character(&costume.color).is_none() { return Err("Choose a valid player color."); }
    let clothing: Vec<(&str, &str)> = [("hat", &costume.hat), ("shirt", &costume.shirt), ("pants", &costume.pants), ("shoes", &costume.shoes), ("gloves", &costume.gloves), ("bag", &costume.bag)]
        .into_iter().filter_map(|(slot, id)| id.as_deref().map(|id| (slot, id)))
        .chain(costume.extras.iter().map(|id| ("extra", id.as_str())))
        .collect();
    for (slot, id) in &clothing {
        if !cosmetics().iter().any(|item| item.id == *id && item.slot == *slot) {
            return Err("Choose an accessory from the catalog.");
        }
    }
    if costume.extras.iter().enumerate().any(|(i, id)| costume.extras[..i].contains(id)) { return Err("Each extra goes on once."); }
    if costume.reaction.iter().chain(&costume.reaction_pool).any(|id| !reactions().iter().any(|reaction| reaction.id == *id)) { return Err("Choose a reaction from the catalog."); }
    if costume.pet.slots().any(|(slot, id)| !pet_items().iter().any(|item| item.id == id && item.slot == slot)) { return Err("Choose pet gear from the catalog."); }
    if costume.tints.iter().any(|(slot, color)| !TINT_SLOTS.contains(&slot.as_str()) || !tint_palette().contains(color)) { return Err("Choose a colour from the palette."); }
    if let Some(id) = costume.model_id.as_deref() {
        // A bag is carried, not worn, so it goes with any character.
        if clothing.iter().any(|(slot, _)| *slot != "bag" && *slot != "extra") { return Err("Full characters can wear extras and carry bags, but not fitted clothing."); }
        if id.is_empty() || id.len() > 100 { return Err("Choose a valid character."); }
        if !cosmetics().iter().any(|item| item.id == id && item.slot == "model")
            && !wardrobe.is_some_and(|wardrobe| wardrobe.costume.model_id.as_deref() == Some(id)) {
            return Err("Choose one of your playable imported models.");
        }
    }
    Ok(())
}

#[cfg(test)]
mod cosmetic_catalog_tests {
    use super::*;

    #[test]
    fn the_clients_copy_of_the_roster_matches() {
        // Profile cards draw characters outside a room, from this copy.
        let copy: Vec<serde_json::Value> = serde_json::from_str(include_str!("../../canvases/src/party/roster.json")).unwrap();
        assert_eq!(copy.len(), ROSTER.len());
        for (copy, character) in copy.iter().zip(ROSTER) {
            assert_eq!(copy, &serde_json::to_value(character).unwrap());
        }
    }

    #[test]
    fn the_registration_catalog_contains_every_purchased_character_once_at_its_price() {
        let mut ids = std::collections::HashSet::new();
        for item in cosmetics() {
            assert!(ids.insert(item.id.as_str()), "duplicate store product {}", item.id);
            assert_eq!(item.price, if item.slot == "model" { 8 } else { 2 });
        }
        for reaction in reactions() {
            assert!(ids.insert(reaction.id.as_str()), "duplicate store product {}", reaction.id);
            assert_eq!(reaction.price, 2);
        }
        assert_eq!(reactions().len(), 10);
        for id in ["model-robot", "model-ninja", "model-cowboy", "model-goblin", "model-knight", "model-pirate"] {
            assert!(ids.contains(id), "{id} must register through the same bot as clothing");
        }
        let earned: Vec<_> = cosmetics().iter().filter(|item| item.unlocked_by.is_some()).map(|item| item.id.as_str()).collect();
        assert_eq!(earned, ["hat-crown", "hat-wizard", "extra-medal", "extra-halo"]);
        assert!(cosmetics().iter().filter(|item| !item.for_sale()).all(|item| item.included_with.is_some() || earned.contains(&item.id.as_str())));
        for item in pet_items() {
            assert!(ids.insert(item.id.as_str()), "duplicate store product {}", item.id);
            assert_eq!(item.price, if item.slot == "pet" { 8 } else { 2 });
        }
        let source: Vec<Cosmetic> = serde_json::from_str(include_str!("../../canvases/src/party/costumes/catalog.json")).unwrap();
        assert_eq!(cosmetics().len(), source.len() + 6);
    }
}
