//! Encore: a band game over a Clone Hero song library. Players plug in a
//! guitar, drums or a keyboard and play their part of the song together.

pub mod access;
pub mod audio;
pub mod chart;
pub mod formats;
pub mod library;
pub mod room;
pub mod runtime;
pub mod scores;
pub mod server;
pub mod store;
pub mod uploads;

pub use server::EncoreServer;
