//! Party: real-time minigames for lobbies. The roster, the room cycle and the
//! minigame trait are shared so the minigames and the party board can reuse
//! them: Bumper Balls on a shrinking platform and Paint Blasters in first
//! person.

pub mod auth;
pub mod balloon;
pub mod board;
pub mod coven;
pub mod landlord;
pub mod crumble;
pub mod dodgeball;
pub mod fpmove;
pub mod ground;
pub mod ledge;
pub mod bout;
pub mod paint_variant;
pub mod potato;
pub mod protocol;
pub mod race;
pub mod redlight;
pub mod room;
pub mod roster;
pub mod runtime;
pub mod server;
pub mod shooter;
pub mod table;
pub mod tank;
pub mod titan;
pub mod workflows;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod coven_socket_tests;
#[cfg(test)]
mod recording_socket_tests;

pub use server::{PartyConfig, PartyServer, RecorderStatus, SessionRecorder};
