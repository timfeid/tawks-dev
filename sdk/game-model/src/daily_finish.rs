//! A daily-game bot reports that a player finished today's game. Tawks decides
//! what that pays; the bot only says who, which game and which puzzle day.
use serde::{Deserialize, Serialize};
use specta::Type;

#[derive(Type, Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
pub struct DailyFinishReport {
    pub user_id: String,
    pub game: String,
    /// The puzzle day as `YYYY-MM-DD`, or None for the day Tawks is on now.
    #[serde(default)]
    pub day: Option<String>,
}
