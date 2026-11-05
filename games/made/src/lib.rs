//! The made relay: `/party/made/ws` on the games server. A lobby's players
//! join with a pass from the `made_join` action; their room runs in made-simd
//! behind a local socket, and the relay streams its scene frames back as
//! deltas. A sim's room (sim_room) shares the socket and the sandbox link.
//! See made_protocol for every message.

pub mod link;
pub mod room;
pub mod runtime;
pub mod server;
pub mod sim_room;
pub mod token;

pub use server::MadeServer;
