//! Rust client types and HTTP clients for Kemono and its forks.
//!
//! - [`kemono`]: generated client for kemono.su / coomer.su (from `schema/kemono.yaml`)
//! - [`pawchive`]: hand-written client for Pawchive (see `docs/pawchive-api.md`)

pub mod client;
pub mod error;
pub mod kemono;
pub mod pawchive;
pub mod types;
