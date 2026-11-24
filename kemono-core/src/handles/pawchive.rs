//! Handles for [`PawchiveClient`]: [`Creator`], [`PostRef`] and [`PostsQuery`].
//!
//! ```no_run
//! # #[cfg(feature = "reqwest")]
//! # async fn demo() -> Result<(), kemono_core::error::Error> {
//! use kemono_core::pawchive::PawchiveClient;
//! let creator = PawchiveClient::default().creator("patreon", "110390090");
//! println!("{}", creator.profile().await?.name);
//! let tags = creator.tags().await?;
//! let mut posts = creator.posts().tag("Steins;Gate").max_pages(2);
//! while let Some(p) = posts.next().await? {
//!     println!("{} {:?}", p.title, p.parsed_tags());
//! }
//! # Ok(()) }
//! ```

use super::paging::Cursor;
use crate::error::Error;
use crate::pawchive::PawchiveClient;
use crate::types::pawchive::{
    PawchiveAnnouncement, PawchiveComment, PawchiveCreatorListItem, PawchiveCreatorProfile,
    PawchiveFancard, PawchiveFavoriteCreator, PawchiveFavoritePost, PawchiveHashPost, PawchivePost,
    PawchivePostSummary, PawchiveRevision, PawchiveTagCount,
};

impl PawchiveClient {
    /// A handle for one creator. No request is made.
    pub fn creator(&self, service: &str, creator_id: &str) -> Creator {
        Creator {
            client: self.clone(),
            service: service.to_string(),
            id: creator_id.to_string(),
        }
    }
}

impl PawchiveClient {
    /// Your favorite creators as handles (needs login). For `faved_seq` and
    /// `last_imported` use [`get_favorite_creators`](Self::get_favorite_creators).
    pub async fn favorite_creators(&self) -> Result<Vec<Creator>, Error> {
        let favs = self.get_favorite_creators().await?;
        Ok(favs.iter().map(|f| f.bind(self)).collect())
    }

    /// Your favorite posts as handles (needs login). For `faved_seq` and the post
    /// data use [`get_favorite_posts`](Self::get_favorite_posts).
    pub async fn favorite_posts(&self) -> Result<Vec<PostRef>, Error> {
        let favs = self.get_favorite_posts().await?;
        Ok(favs.iter().map(|f| f.bind(self)).collect())
    }
}

macro_rules! bind_creator {
    ($($t:ty => $svc:ident, $id:ident;)*) => {$(
        impl $t {
            /// `(service, id)`, the creator's identity.
            pub fn key(&self) -> (&str, &str) {
                (&self.$svc, &self.$id)
            }

            /// Turns this into a [`Creator`] handle on `client`.
            pub fn bind(&self, client: &PawchiveClient) -> Creator {
                client.creator(&self.$svc, &self.$id)
            }
        }
    )*};
}

macro_rules! bind_post {
    ($($t:ty => $($path:ident).+;)*) => {$(
        impl $t {
            /// A [`PostRef`] handle (post and creator) on `client`.
            pub fn bind(&self, client: &PawchiveClient) -> PostRef {
                let p = &self.$($path).+;
                client.creator(&p.service, &p.user).post(&p.id)
            }
        }
    )*};
}

bind_creator! {
    PawchiveCreatorListItem => service, id;
}
bind_post! {
    PawchiveFavoritePost => post;
    PawchiveRevision => post;
}

impl PawchiveFavoriteCreator {
    /// `(service, id)`, the creator's identity.
    pub fn key(&self) -> (&str, &str) {
        self.profile.key()
    }

    /// Turns this into a [`Creator`] handle on `client`.
    pub fn bind(&self, client: &PawchiveClient) -> Creator {
        self.profile.bind(client)
    }
}

impl PawchivePostSummary {
    /// A [`PostRef`] handle on `client`.
    pub fn bind(&self, client: &PawchiveClient) -> PostRef {
        client.creator(&self.service, &self.user).post(&self.id)
    }
}

impl PawchiveHashPost {
    /// A [`PostRef`] handle on `client`.
    pub fn bind(&self, client: &PawchiveClient) -> PostRef {
        client.creator(&self.service, &self.user).post(&self.id)
    }
}

impl PawchiveCreatorProfile {
    /// `(service, id)`, the creator's identity.
    pub fn key(&self) -> (&str, &str) {
        (&self.service, &self.id)
    }

    /// Turns this profile into a handle on `client`.
    pub fn bind(&self, client: &PawchiveClient) -> Creator {
        client.creator(&self.service, &self.id)
    }
}

impl PawchivePost {
    /// A handle on this post (and its creator) on `client`.
    pub fn bind(&self, client: &PawchiveClient) -> PostRef {
        client.creator(&self.service, &self.user).post(&self.id)
    }
}

/// A creator on Pawchive.
#[derive(Debug, Clone)]
pub struct Creator {
    client: PawchiveClient,
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

    pub async fn profile(&self) -> Result<PawchiveCreatorProfile, Error> {
        self.client.get_profile(&self.service, &self.id).await
    }

    pub async fn tags(&self) -> Result<Vec<PawchiveTagCount>, Error> {
        self.client.get_tags(&self.service, &self.id).await
    }

    /// Linked accounts of the creator.
    pub async fn links(&self) -> Result<Vec<PawchiveCreatorProfile>, Error> {
        self.client.get_links(&self.service, &self.id).await
    }

    pub async fn announcements(&self) -> Result<Vec<PawchiveAnnouncement>, Error> {
        self.client.get_announcements(&self.service, &self.id).await
    }

    pub async fn fancards(&self) -> Result<Vec<PawchiveFancard>, Error> {
        self.client.get_fancards(&self.service, &self.id).await
    }

    /// Query this creator's posts; see [`PostsQuery`].
    pub fn posts(&self) -> PostsQuery {
        PostsQuery {
            creator: self.clone(),
            tag: None,
            q: None,
            cursor: Cursor::new(None),
            max_pages: None,
        }
    }

    /// A handle on one of this creator's posts. No request is made.
    pub fn post(&self, post_id: &str) -> PostRef {
        PostRef {
            creator: self.clone(),
            id: post_id.to_string(),
        }
    }

    /// Needs login. Checks that the creator exists first.
    pub async fn favorite(&self) -> Result<(), Error> {
        self.client
            .add_favorite_creator(&self.service, &self.id)
            .await
    }

    /// Needs login.
    pub async fn unfavorite(&self) -> Result<(), Error> {
        self.client
            .remove_favorite_creator(&self.service, &self.id)
            .await
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

    pub async fn fetch(&self) -> Result<PawchivePost, Error> {
        let c = &self.creator;
        c.client.get_post(&c.service, &c.id, &self.id).await
    }

    /// Empty when there are none.
    pub async fn comments(&self) -> Result<Vec<PawchiveComment>, Error> {
        let c = &self.creator;
        c.client.get_comments(&c.service, &c.id, &self.id).await
    }

    /// Newest first; empty when there are none.
    pub async fn revisions(&self) -> Result<Vec<PawchiveRevision>, Error> {
        let c = &self.creator;
        c.client.get_revisions(&c.service, &c.id, &self.id).await
    }

    /// Needs login. Checks that the post exists first.
    pub async fn favorite(&self) -> Result<(), Error> {
        let c = &self.creator;
        c.client
            .add_favorite_post(&c.service, &c.id, &self.id)
            .await
    }

    /// Needs login.
    pub async fn unfavorite(&self) -> Result<(), Error> {
        let c = &self.creator;
        c.client
            .remove_favorite_post(&c.service, &c.id, &self.id)
            .await
    }
}

/// A creator's posts, 50 per page, with optional filters.
///
/// Use [`page`](Self::page) for one page, [`next`](Self::next) to walk posts lazily,
/// or [`collect`](Self::collect) for everything. `max_pages` bounds `next`/`collect`
/// (a large creator can mean hundreds of requests).
#[derive(Debug)]
pub struct PostsQuery {
    creator: Creator,
    tag: Option<String>,
    q: Option<String>,
    max_pages: Option<u32>,
    cursor: Cursor<PawchivePost>,
}

impl PostsQuery {
    /// Exact tag name.
    pub fn tag(mut self, tag: &str) -> Self {
        self.tag = Some(tag.to_string());
        self
    }

    /// Full-text search.
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
    pub async fn page(&self, n: u32) -> Result<Vec<PawchivePost>, Error> {
        self.fetch(n * super::paging::PAGE_SIZE).await
    }

    /// The next post, fetching pages as needed; `None` at the end.
    pub async fn next(&mut self) -> Result<Option<PawchivePost>, Error> {
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
    pub async fn collect(mut self) -> Result<Vec<PawchivePost>, Error> {
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

    async fn fetch(&self, offset: u32) -> Result<Vec<PawchivePost>, Error> {
        Self::fetch_with(&self.creator, &self.tag, &self.q, offset).await
    }

    async fn fetch_with(
        creator: &Creator,
        tag: &Option<String>,
        q: &Option<String>,
        offset: u32,
    ) -> Result<Vec<PawchivePost>, Error> {
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
