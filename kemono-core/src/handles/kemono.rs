//! Handles for [`KemonoClient`]: [`Creator`], [`PostRef`] and [`PostsQuery`].
//!
//! Only what the Kemono API schema offers: a creator's posts (tag filter, search,
//! paging) and single posts. Profiles and tags have no endpoint in `schema/kemono.yaml`.
//!
//! ```no_run
//! # #[cfg(feature = "reqwest")]
//! # async fn demo() -> Result<(), kemono_core::error::Error> {
//! use kemono_core::kemono::KemonoClient;
//! let creator = KemonoClient::new("https://kemono.su/api").creator("patreon", "110390090");
//! let mut posts = creator.posts().max_pages(1);
//! while let Some(p) = posts.next().await? {
//!     println!("{}", p.title);
//! }
//! # Ok(()) }
//! ```

use super::paging::{Cursor, PAGE_SIZE};
use crate::error::Error;
use crate::kemono::KemonoClient;
use crate::types::post::{PostRoot, PostSummary};

impl KemonoClient {
    /// A handle for one creator. No request is made.
    pub fn creator(&self, service: &str, creator_id: &str) -> Creator {
        Creator {
            client: self.clone(),
            service: service.to_string(),
            id: creator_id.to_string(),
        }
    }
}

impl PostSummary {
    /// A handle on this post (and its creator) on `client`.
    pub fn bind(&self, client: &KemonoClient) -> PostRef {
        client.creator(&self.service, &self.user).post(&self.id)
    }
}

/// A creator on Kemono/Coomer.
#[derive(Debug, Clone)]
pub struct Creator {
    client: KemonoClient,
    service: String,
    id: String,
}

impl Creator {
    pub fn service(&self) -> &str {
        &self.service
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// Query this creator's posts; see [`PostsQuery`].
    pub fn posts(&self) -> PostsQuery {
        PostsQuery {
            creator: self.clone(),
            tag: None,
            q: None,
            max_pages: None,
            cursor: Cursor::new(None),
        }
    }

    /// A handle on one of this creator's posts. No request is made.
    pub fn post(&self, post_id: &str) -> PostRef {
        PostRef {
            creator: self.clone(),
            id: post_id.to_string(),
        }
    }
}

/// One post of a [`Creator`].
#[derive(Debug, Clone)]
pub struct PostRef {
    creator: Creator,
    id: String,
}

impl PostRef {
    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn creator(&self) -> &Creator {
        &self.creator
    }

    pub async fn fetch(&self) -> Result<PostRoot, Error> {
        let c = &self.creator;
        c.client.get_post_detail(&c.service, &c.id, &self.id).await
    }
}

/// A creator's posts, 50 per page, with optional filters. Same usage as
/// [`pawchive::PostsQuery`](super::pawchive::PostsQuery).
#[derive(Debug)]
pub struct PostsQuery {
    creator: Creator,
    tag: Option<String>,
    q: Option<String>,
    max_pages: Option<u32>,
    cursor: Cursor<PostSummary>,
}

impl PostsQuery {
    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    pub fn query(mut self, q: &str) -> Self {
        self.q = Some(q.to_string());
        self
    }

    /// Cap on requests made by `next`/`collect`.
    pub fn max_pages(mut self, n: u32) -> Self {
        self.max_pages = Some(n);
        self.cursor = Cursor::new(self.max_pages);
        self
    }

    /// Page `n` (0-based) on its own, independent of `next`.
    pub async fn page(&self, n: u32) -> Result<Vec<PostSummary>, Error> {
        Self::fetch_with(&self.creator, &self.tag, &self.q, n * PAGE_SIZE).await
    }

    /// The next post, fetching pages as needed; `None` at the end.
    pub async fn next(&mut self) -> Result<Option<PostSummary>, Error> {
        let Self {
            creator,
            tag,
            q,
            cursor,
            ..
        } = self;
        cursor.next(|o| Self::fetch_with(creator, tag, q, o)).await
    }

    /// All remaining posts.
    pub async fn collect(mut self) -> Result<Vec<PostSummary>, Error> {
        let Self {
            creator,
            tag,
            q,
            cursor,
            ..
        } = &mut self;
        cursor
            .collect(|o| Self::fetch_with(creator, tag, q, o))
            .await
    }

    async fn fetch_with(
        creator: &Creator,
        tag: &Option<String>,
        q: &Option<String>,
        offset: u32,
    ) -> Result<Vec<PostSummary>, Error> {
        creator
            .client
            .get_user_posts(
                &creator.service,
                &creator.id,
                tag.as_deref(),
                Some(offset),
                q.as_deref(),
            )
            .await
    }
}
