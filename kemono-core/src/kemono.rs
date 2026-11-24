//! Client for Kemono (kemono.su) and Coomer (coomer.su); see `schema/kemono.yaml`.
//!
//! The low-level methods here take `(service, creator_id)` strings; for an
//! object-style API use [`KemonoClient::creator`] (see [`crate::handles::kemono`]).

use std::sync::Arc;

use crate::client::{HttpCore, RetryPolicy};
use crate::error::Error;
use crate::transport::Transport;
use crate::types::post::{PostRoot, PostSummary};

#[derive(Debug, Clone)]
pub struct KemonoClient {
    http: HttpCore,
}

impl KemonoClient {
    /// Creates a new client on [`ReqwestTransport`](crate::transport::ReqwestTransport) (feature `reqwest`).
    /// The `base_url` can be `https://kemono.su/api` or `https://coomer.su/api`.
    #[cfg(feature = "reqwest")]
    pub fn new(base_url: &str) -> Self {
        Self::with_transport(
            base_url,
            Arc::new(crate::transport::ReqwestTransport::new()),
        )
    }

    /// A client that sends its requests through `transport`.
    pub fn with_transport(base_url: &str, transport: Arc<dyn Transport>) -> Self {
        Self {
            http: HttpCore::new(base_url, transport).with_header("Accept", "text/css"),
        }
    }

    /// Replaces the retry policy (default: 3 attempts, see [`RetryPolicy`]).
    pub fn with_retry(mut self, policy: RetryPolicy) -> Self {
        self.http = self.http.with_retry(policy);
        self
    }

    /// A creator's posts, 50 per page.
    /// Corresponds to `GET /v1/{service}/user/{creator_id}/posts`
    ///
    /// `offset` must be a multiple of [`PAGE_SIZE`](crate::handles::paging::PAGE_SIZE).
    pub async fn get_user_posts(
        &self,
        service: &str,
        creator_id: &str,
        tag: Option<&str>,
        offset: Option<u32>,
        q: Option<&str>,
    ) -> Result<Vec<PostSummary>, Error> {
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

    /// Get a specific post by its ID.
    /// Corresponds to `GET /v1/{service}/user/{creator_id}/post/{post_id}`
    pub async fn get_post_detail(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<PostRoot, Error> {
        let url = format!(
            "{}/v1/{}/user/{}/post/{}",
            self.http.base_url(),
            service,
            creator_id,
            post_id
        );
        self.http.get_json(url, &[]).await
    }
}
