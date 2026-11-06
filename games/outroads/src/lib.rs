//! Outroads: drive around the real world with friends in a tawks lobby. The
//! client draws OpenStreetMap roads and buildings from vector tiles; this
//! crate keeps each lobby's spawn point, votes and car poses, and searches
//! for places on the players' behalf.

pub mod geocode;
pub mod pass;
pub mod room;
pub mod runtime;
pub mod server;

pub use server::OutroadsServer;
