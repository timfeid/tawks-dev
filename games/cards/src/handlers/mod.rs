//! Action handlers for the card game
//!
//! This module contains trait-based handlers that organize the game's action processing
//! logic by domain (lobby, combat, builder, etc.). Each handler trait is implemented on
//! CardGameState and provides methods for handling specific game actions.

// Simple handlers (Phase 3)
mod ai;
mod cards;
mod choices;
mod stack;
mod utility;

// Medium handlers (Phase 4)
mod game_start;
mod lobby;
mod mana;
mod match_flow;
mod spells;

// Complex handlers (Phase 5)
mod builder;
mod combat;

// Draft
mod draft;

// Packs
mod packs;

// Debug
mod debug;

// Re-exports
pub use cards::*;
pub use choices::*;
pub use stack::*;
pub use utility::*;

pub use game_start::*;
pub use lobby::*;
pub use mana::*;
pub use match_flow::*;
pub use spells::*;

pub use builder::*;
pub use combat::*;
pub use debug::*;
pub(crate) use draft::choose_auto_draft_pick;
pub use draft::*;
pub use packs::*;
