//! Types for Pawchive (a Kemono fork). Shapes follow `docs/pawchive-api.md`.

pub mod comment;
pub mod creator;
pub mod hash;
pub mod post;

pub use comment::{PawchiveComment, PawchiveCommentRevision};
pub use creator::{
    PawchiveAnnouncement, PawchiveCreatorListItem, PawchiveCreatorProfile, PawchiveFancard,
    PawchiveFavoriteCreator, PawchiveTagCount,
};
pub use hash::{PawchiveHashFile, PawchiveHashPost, PawchiveHashSearch};
pub use post::{
    PawchiveFavoritePost, PawchiveFile, PawchivePost, PawchivePostSummary, PawchiveRevision,
    RawTags,
};
