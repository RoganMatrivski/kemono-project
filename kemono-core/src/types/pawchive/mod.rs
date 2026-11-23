//! Types for Pawchive (a Kemono fork). Shapes follow `docs/pawchive-api.md`.

pub mod comment;
pub mod creator;
pub mod hash;
pub mod post;

pub use comment::{PawchiveComment, PawchiveCommentRevision};
pub use creator::{
    PawchiveCreatorListItem, PawchiveCreatorProfile, PawchiveFancard, PawchiveTagCount,
};
pub use hash::{PawchiveHashFile, PawchiveHashPost, PawchiveHashSearch};
pub use post::{PawchiveFile, PawchivePost, PawchivePostSummary, PawchiveRevision};
