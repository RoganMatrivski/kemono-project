//! Object-style API on top of the clients.
//!
//! A *handle* is a cheap clone of a client plus an identity (`service`, `id`), so
//! you can write `client.creator(s, id).posts().collect()` instead of passing the
//! same strings to every client method. Handles do no I/O until awaited.
//!
//! - [`pawchive`]: [`pawchive::Creator`], [`pawchive::PostRef`], [`pawchive::PostsQuery`]
//! - [`kemono`]: the same shapes for [`KemonoClient`](crate::kemono::KemonoClient)

pub mod kemono;
pub(crate) mod paging;
pub mod pawchive;
