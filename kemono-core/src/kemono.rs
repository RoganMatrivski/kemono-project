//! This file is generated from kemono.yaml. Do not edit.

use reqwest::Client;
use serde::Deserialize;

use crate::types::post::{PostRoot, PostSummary};

#[derive(Debug)]
pub struct KemonoClient {
    client: Client,
    base_url: String,
}

impl KemonoClient {
    /// Creates a new client.
    /// The `base_url` can be "https://kemono.su/api" or "https://coomer.su/api".
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .user_agent("kemono-rs/0.1.0")
                .default_headers(
                    std::iter::once((reqwest::header::ACCEPT, "text/css".parse().unwrap()))
                        .collect(),
                )
                .gzip(true)
                .build()
                .unwrap(),
            base_url: base_url.to_string(),
        }
    }

    /// Get posts from a specific creator.
    /// Corresponds to `GET /v1/{service}/user/{creator_id}/posts`
    pub async fn get_user_posts(
        &self,
        service: &str,
        creator_id: &str,
        tag: Option<&str>,
    ) -> Result<Vec<PostSummary>, reqwest::Error> {
        let url = format!("{}/v1/{}/user/{}/posts", self.base_url, service, creator_id);

        let mut query_params = Vec::new();
        if let Some(t) = tag {
            query_params.push(("tag", t));
        }

        self.client
            .get(url)
            .query(&query_params)
            .send()
            .await?
            .json::<Vec<PostSummary>>()
            .await
    }

    /// Get a specific post by its ID.
    /// Corresponds to `GET /v1/{service}/user/{creator_id}/post/{post_id}`
    pub async fn get_post_detail(
        &self,
        service: &str,
        creator_id: &str,
        post_id: &str,
    ) -> Result<PostRoot, reqwest::Error> {
        let url = format!(
            "{}/v1/{}/user/{}/post/{}",
            self.base_url, service, creator_id, post_id
        );

        self.client.get(url).send().await?.json::<PostRoot>().await
    }
}
