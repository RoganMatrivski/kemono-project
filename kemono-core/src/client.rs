//! HTTP plumbing shared by the fork clients (currently only Pawchive).

use reqwest::Client;
use serde::de::DeserializeOwned;

use crate::error::Error;

/// A `reqwest::Client` plus a base URL, with status-to-[`Error`] mapping.
#[derive(Debug, Clone)]
pub struct HttpCore {
    client: Client,
    base_url: String,
}

impl HttpCore {
    /// `base_url` like "https://pawchive.pw/api"; a trailing slash is trimmed.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent("kemono-rs/0.1.0")
            .gzip(true)
            .build()
            .unwrap();
        Self::from_client(client, base_url)
    }

    /// Use a pre-configured client (headers, cookie store, ...).
    pub fn from_client(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// GET `url`, map non-2xx to [`Error`], decode the JSON body.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: String,
        params: &[(&str, String)],
    ) -> Result<T, Error> {
        let resp = self.client.get(url).query(params).send().await?;
        Ok(Error::check(resp).await?.json().await?)
    }
}
