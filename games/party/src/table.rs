//! Table games: turn-based card games played round the party table. Unlike
//! the minigames they never tick physics, they carry hidden information, so
//! every seat gets its own view, and they seat more than an arena does.

use serde_json::Value;

pub trait TableGame: Send {
    fn key(&self) -> &'static str;
    /// One seat acts; the error is the sentence that seat should read.
    fn act(&mut self, seat: u8, action: Value) -> Result<(), String>;
    fn tick(&mut self, dt: f32);
    /// Settles the current phase the way its clock would.
    fn hurry(&mut self);
    /// A practice bot takes its turn; the default is to let the clock decide.
    fn bot_act(&mut self) {
        self.hurry();
    }
    fn pending_actors(&self) -> Vec<u8>;
    fn view_for(&self, seat: Option<u8>) -> Value;
    fn log_len(&self) -> usize;
    /// The seats that won, once the game is over.
    fn winners(&self) -> Option<Vec<u8>>;
}

pub struct TableSpec {
    pub key: &'static str,
    pub name: &'static str,
    pub min_players: usize,
    pub max_players: usize,
    pub create: fn(usize, u64) -> Box<dyn TableGame>,
}

pub const TABLE_SPECS: &[TableSpec] = &[
    TableSpec {
        key: crate::coven::KEY,
        name: "Coven",
        min_players: crate::coven::MIN_PLAYERS,
        max_players: crate::coven::MAX_PLAYERS,
        create: |players, seed| Box::new(crate::coven::Coven::new(players, seed)),
    },
    TableSpec {
        key: crate::landlord::KEY,
        name: "Landlord",
        min_players: crate::landlord::MIN_PLAYERS,
        max_players: crate::landlord::MAX_PLAYERS,
        create: |players, seed| Box::new(crate::landlord::Landlord::new(players, seed)),
    },
];

/// Key and name pairs for the room view, in the same order as the specs.
pub const TABLE_GAMES: &[(&str, &str)] = &[(crate::coven::KEY, "Coven"), (crate::landlord::KEY, "Landlord")];

pub fn spec(key: &str) -> Option<&'static TableSpec> {
    TABLE_SPECS.iter().find(|s| s.key == key)
}
