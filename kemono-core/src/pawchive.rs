//! Client for Pawchive (https://pawchive.pw), a Kemono fork.

use crate::client::HttpCore;
use crate::error::Error;
use crate::types::pawchive::{
    PawchiveComment, PawchiveCreatorListItem, PawchiveCreatorProfile, PawchiveFancard,
    PawchiveHashSearch, PawchivePost, PawchivePostSummary, PawchiveRevision, PawchiveTagCount,
};

/// Pawchive serves 50 results per page; `o` must be a multiple of this.
pub const PAGE_SIZE: u32 = 50;

#[derive(Debug)]
pub struct PawchiveClient {
    http: HttpCore,
}

impl Default for PawchiveClient {
    fn default() -> Self {
        Self::new("https://pawchive.pw/api")
    }
}

impl PawchiveClient {
    /// `base_url` is e.g. "https://pawchive.pw/api" (no trailing slash).
    pub fn new(base_url: &str) -> Self {
        Self {
            http: HttpCore::new(base_url),
        }
    }

    /// A creator's posts (full shape), 50 per page.
    /// `GET /v1/{service}/user/{creator_id}/posts`
    ///
    /// `offset` must be a multiple of [`PAGE_SIZE`]. `tag` is an exact tag name.
    /// `q` is a full-text search; no match yields an empty list.
    pub async fn get_user_posts(
        &self,
        service: &str,
        creator_id: &str,
        tag: Option<&str>,
        offset: Option<u32>,
        q: Option<&str>,
    ) -> Result<Vec<PawchivePost>, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/posts",
            self.http.base_url(),
            service,
            creator_id
        );
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(t) = tag {
            params.push(("tag", t.to_string()));
        }
        if let Some(o) = offset {
            params.push(("o", o.to_string()));
        }
        if let Some(q) = q {
            params.push(("q", q.to_string()));
        }
        self.http.get_json(url, &params).await
    }

    /// `GET /v1/{service}/user/{creator_id}/post/{post_id}`
    pub async fn get_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<PawchivePost, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/post/{}",
            self.http.base_url(),
            service,
            creator_id,
            post_id
        );
        self.http.get_json(url, &[]).await
    }

    /// Recent posts site-wide (listing shape), 50 per page. `GET /v1/posts`
    pub async fn get_posts(
        &self,
        q: Option<&str>,
        offset: Option<u32>,
    ) -> Result<Vec<PawchivePostSummary>, Error> {
        let mut params: Vec<(&str, String)> = Vec::new();
        if let Some(q) = q {
            params.push(("q", q.to_string()));
        }
        if let Some(o) = offset {
            params.push(("o", o.to_string()));
        }
        self.http
            .get_json(format!("{}/v1/posts", self.http.base_url()), &params)
            .await
    }

    /// Every creator on the site in one response (~16 MB). `GET /v1/creators`
    pub async fn get_creators(&self) -> Result<Vec<PawchiveCreatorListItem>, Error> {
        self.http
            .get_json(format!("{}/v1/creators", self.http.base_url()), &[])
            .await
    }

    fn user_url(&self, service: &str, creator_id: &str, tail: &str) -> String {
        format!(
            "{}/v1/{}/user/{}/{}",
            self.http.base_url(),
            service,
            creator_id,
            tail
        )
    }

    /// `GET /v1/{service}/user/{creator_id}/tags`
    pub async fn get_tags(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveTagCount>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "tags"), &[])
            .await
    }

    /// Linked accounts of the creator. `GET /v1/{service}/user/{creator_id}/links`
    pub async fn get_links(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveCreatorProfile>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "links"), &[])
            .await
    }

    /// Announcements; the item shape is unknown (every sampled creator had none), so raw JSON.
    /// `GET /v1/{service}/user/{creator_id}/announcements`
    pub async fn get_announcements(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<serde_json::Value>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "announcements"), &[])
            .await
    }

    /// Fanbox fancards (other services return `[]`). `GET /v1/{service}/user/{creator_id}/fancards`
    pub async fn get_fancards(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<Vec<PawchiveFancard>, Error> {
        self.http
            .get_json(self.user_url(service, creator_id, "fancards"), &[])
            .await
    }

    /// Comments on a post; the server's 404 for "none" becomes an empty list.
    pub async fn get_comments(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PawchiveComment>, Error> {
        let url = self.user_url(service, creator_id, &format!("post/{post_id}/comments"));
        empty_on_404(self.http.get_json(url, &[]).await)
    }

    /// Revisions, newest first; the server's 404 for "none" becomes an empty list.
    pub async fn get_revisions(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<Vec<PawchiveRevision>, Error> {
        let url = self.user_url(service, creator_id, &format!("post/{post_id}/revisions"));
        empty_on_404(self.http.get_json(url, &[]).await)
    }

    /// Posts containing a file with this SHA-256. `GET /v1/search_hash/{sha256}`
    pub async fn search_hash(&self, sha256: &str) -> Result<PawchiveHashSearch, Error> {
        self.http
            .get_json(
                format!("{}/v1/search_hash/{}", self.http.base_url(), sha256),
                &[],
            )
            .await
    }

    /// `GET /v1/{service}/user/{creator_id}/profile`
    pub async fn get_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<PawchiveCreatorProfile, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/profile",
            self.http.base_url(),
            service,
            creator_id
        );
        self.http.get_json(url, &[]).await
    }
}

fn empty_on_404<T>(r: Result<Vec<T>, Error>) -> Result<Vec<T>, Error> {
    match r {
        Err(Error::NotFound) => Ok(Vec::new()),
        other => other,
    }
}

#[cfg(test)]
mod live {
    use super::*;

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_roundtrip() {
        let c = PawchiveClient::default();
        let prof = c.get_profile("patreon", "110390090").await.unwrap();
        assert_eq!(prof.id, "110390090");
        let posts = c
            .get_user_posts("patreon", "110390090", None, Some(0), None)
            .await
            .unwrap();
        assert!(!posts.is_empty());
        let p = c
            .get_post("patreon", "110390090", "176121337")
            .await
            .unwrap();
        assert_eq!(p.id, "176121337");
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_error_mapping() {
        let c = PawchiveClient::default();
        let r = c
            .get_user_posts("patreon", "110390090", None, Some(25), None)
            .await;
        assert!(matches!(r, Err(Error::BadRequest(_))), "{r:?}");
        let r = c.get_post("patreon", "110390090", "0").await;
        assert!(matches!(r, Err(Error::NotFound)), "{r:?}");
    }

    #[tokio::test]
    #[ignore = "hits pawchive.pw"]
    async fn live_read_only() {
        let c = PawchiveClient::default();
        assert_eq!(c.get_posts(None, Some(0)).await.unwrap().len(), 50);
        let tags = c.get_tags("patreon", "110390090").await.unwrap();
        assert!(!tags.is_empty());
        let links = c.get_links("fanbox", "37736420").await.unwrap();
        assert!(links.iter().any(|l| l.relation_id.is_some()));
        assert!(
            !c.get_fancards("fanbox", "37736420")
                .await
                .unwrap()
                .is_empty()
        );
        c.get_announcements("patreon", "110390090").await.unwrap();
        // no comments / revisions -> empty, not an error
        assert!(
            c.get_comments("patreon", "110390090", "176121337")
                .await
                .unwrap()
                .is_empty()
        );
        assert!(
            c.get_revisions("patreon", "110390090", "176121337")
                .await
                .unwrap()
                .is_empty()
        );
        let found = c
            .get_comments("patreon", "90235213", "176154456")
            .await
            .unwrap();
        assert!(!found.is_empty());
        let h = "44ad9682c02ecfec5b55e0d06cbba9d4d9d11d10895ee859be2588a714d33577";
        let res = c.search_hash(h).await.unwrap();
        assert!(res.posts[0].file.url().is_some());
        let all = c.get_creators().await.unwrap();
        assert!(all.len() > 90_000);
    }
}
