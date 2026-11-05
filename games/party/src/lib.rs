//! Party: real-time minigames for lobbies. The roster, the room cycle and the
//! minigame trait are shared so the minigames and the party board can reuse
//! them: Bumper Balls on a shrinking platform and Paint Blasters in first
//! person.

pub mod auth;
pub mod balloon;
pub mod blueprints;
pub mod blueprints_catalog;
pub mod board;
mod buzzer;
pub mod coins;
pub mod city_arrest;
pub mod city_slip;
pub mod city_buildings;
pub mod city_heist;
pub mod coven;
pub mod crowd_current;
pub mod design;
pub mod diner;
pub mod landlord;
pub mod lastfloor;
pub mod crumble;
pub mod dodgeball;
pub mod dockside_tangle;
pub mod echo;
pub mod feedback;
pub mod fpmove;
pub mod fuse;
pub mod flux_skip;
pub mod gallery;
pub mod ghosts;
pub mod glass;
pub mod ground;
mod hill;
pub mod ledge;
pub mod lineup;
pub mod lantern;
pub mod cue_crowd;
pub mod parcel_panic;
pub mod whirligig;
pub mod chimney_dash;
pub mod garden_snapshot;
pub mod memory;
pub mod margin_run;
pub mod awards;
pub mod milestones;
pub mod minecart;
pub mod reef_bloom;
pub mod mugshot;
pub mod bout;
pub mod bridgecrew;
pub mod paint_variant;
pub mod pets;
pub mod potato;
pub mod protocol;
pub mod race;
pub mod ravine_draft;
pub mod redlight;
pub mod rising;
pub mod relay;
pub mod room;
pub mod roster;
pub mod runtime;
pub mod server;
pub mod shooter;
pub mod speedball;
pub mod stencil;
pub mod stackbreak_yard;
pub mod signal_sweep;
pub mod shadow_stamp;
pub mod kite_net;
pub mod mixer_circuit;
pub mod sumo;
pub mod table;
pub mod tank;
pub mod titan;
pub mod teeter;
pub mod beacon;
pub mod pigeon;
pub mod rime;
pub mod orbit;
pub mod wake;
pub mod powder;
pub mod mirth;
pub mod pet_arcade;
pub mod pet_arena;
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
