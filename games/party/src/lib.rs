//! Party: real-time minigames for lobbies. The roster, the room cycle and the
//! minigame trait are shared so the minigames and the party board can reuse
//! them: Bumper Balls on a shrinking platform and Paint Blasters in first
//! person.

pub mod auth;
pub mod balloon;
pub mod board;
mod buzzer;
pub mod coins;
pub mod city_heist;
pub mod coven;
pub mod design;
pub mod landlord;
pub mod crumble;
pub mod dodgeball;
pub mod feedback;
pub mod fpmove;
pub mod ghosts;
pub mod ground;
mod hill;
pub mod ledge;
pub mod memory;
pub mod milestones;
pub mod minecart;
pub mod bout;
pub mod paint_variant;
pub mod potato;
pub mod protocol;
pub mod race;
pub mod redlight;
pub mod rising;
pub mod relay;
pub mod room;
pub mod roster;
pub mod runtime;
pub mod server;
pub mod shooter;
pub mod speedball;
pub mod sumo;
pub mod table;
pub mod tank;
pub mod titan;
pub mod tug;
pub mod workflows;

#[cfg(test)]
mod tests;
#[cfg(test)]
mod coven_socket_tests;
#[cfg(test)]
mod recording_socket_tests;
#[cfg(test)]
mod board_map_tests;
#[cfg(test)]
mod sandbox_costume_tests;

pub use server::{PartyConfig, PartyServer, RecorderStatus, SessionRecorder};

#[cfg(test)]
mod feedback_delivery_tests;
#[cfg(test)]
mod party_sim_tests;
