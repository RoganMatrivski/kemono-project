//! Rust client types and HTTP clients for Kemono and its forks.
//!
//! - [`kemono`]: client for kemono.su / coomer.su (see `schema/kemono.yaml`)
//! - [`pawchive`]: hand-written client for Pawchive (see `docs/pawchive-api.md`)
//! - [`handles`]: `client.creator(..).posts()`-style API over both clients

pub mod client;
pub mod error;
pub mod handles;
pub mod kemono;
pub mod pawchive;
pub mod types;
