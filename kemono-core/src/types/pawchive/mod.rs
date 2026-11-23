//! Types for Pawchive (a Kemono fork). Shapes follow `docs/pawchive-api.md`.

pub mod creator;
pub mod post;

pub use creator::PawchiveCreatorProfile;
pub use post::{PawchiveFile, PawchivePost, PawchivePostSummary};
