//! Client for Pawchive (https://pawchive.pw), a Kemono fork.

use reqwest::Client;

use crate::types::pawchive::{PawchiveCreatorProfile, PawchivePost};

/// Pawchive serves 50 results per page; `o` must be a multiple of this.
pub const PAGE_SIZE: u32 = 50;

#[derive(Debug)]
pub struct PawchiveClient {
    client: Client,
    base_url: String,
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
            client: Client::builder()
                .user_agent("kemono-rs/0.1.0")
                .gzip(true)
                .build()
                .unwrap(),
            base_url: base_url.trim_end_matches('/').to_string(),
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
    ) -> Result<Vec<PawchivePost>, reqwest::Error> {
        let url = format!("{}/v1/{}/user/{}/posts", self.base_url, service, creator_id);
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
        self.client
            .get(url)
            .query(&params)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    /// `GET /v1/{service}/user/{creator_id}/post/{post_id}`
    pub async fn get_post(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<PawchivePost, reqwest::Error> {
        let url = format!(
            "{}/v1/{}/user/{}/post/{}",
            self.base_url, service, creator_id, post_id
        );
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
    }

    /// `GET /v1/{service}/user/{creator_id}/profile`
    pub async fn get_profile(
        &self,
        service: &str,
        creator_id: &str,
    ) -> Result<PawchiveCreatorProfile, reqwest::Error> {
        let url = format!(
            "{}/v1/{}/user/{}/profile",
            self.base_url, service, creator_id
        );
        self.client
            .get(url)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await
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
        let p = c.get_post("patreon", "110390090", "176121337").await.unwrap();
        assert_eq!(p.id, "176121337");
    }
}
