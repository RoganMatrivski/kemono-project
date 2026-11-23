//! HTTP plumbing shared by the fork clients (currently only Pawchive).
//!
//! [`HttpCore`] wraps a `reqwest::Client` and a base URL and maps HTTP statuses to
//! [`Error`] so each fork client only has to build paths and types. Requests made
//! through it (`GET`, and the idempotent no-content calls) are retried with backoff.

use std::time::Duration;

use reqwest::{Client, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;

use crate::error::{Error, retry_after_secs};

/// Default per-request timeout.
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// A server-sent `Retry-After` is never waited on for longer than this.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(30);

/// How failed requests are retried: connection errors, timeouts, 429 and 5xx.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RetryPolicy {
    /// Total attempts including the first (1 = never retry).
    pub max_attempts: u32,
    /// Delay before the first retry; doubles on every further retry.
    pub base_delay: Duration,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(500),
        }
    }
}

impl RetryPolicy {
    /// No retries.
    pub const NONE: RetryPolicy = RetryPolicy {
        max_attempts: 1,
        base_delay: Duration::ZERO,
    };

    /// Delay before retry number `retry` (0 = first retry). A `Retry-After` wins, capped at 30 s.
    pub fn delay(&self, retry: u32, retry_after: Option<u64>) -> Duration {
        match retry_after {
            Some(s) => Duration::from_secs(s).min(MAX_RETRY_AFTER),
            None => self
                .base_delay
                .saturating_mul(2u32.saturating_pow(retry.min(16))),
        }
    }
}

/// A `reqwest::Client` plus a base URL, with status-to-[`Error`] mapping and retries.
#[derive(Debug, Clone)]
pub struct HttpCore {
    client: Client,
    base_url: String,
    retry: RetryPolicy,
}

impl HttpCore {
    /// `base_url` like `https://pawchive.pw/api`; a trailing slash is trimmed.
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .user_agent("kemono-rs/0.1.0")
            .gzip(true)
            .timeout(DEFAULT_TIMEOUT)
            .build()
            .unwrap();
        Self::from_client(client, base_url)
    }

    /// Use a pre-configured client (headers, cookie store, timeout, ...).
    pub fn from_client(client: Client, base_url: &str) -> Self {
        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
            retry: RetryPolicy::default(),
        }
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    /// Send a request built by `build`, retrying per the policy. `build` runs once per attempt.
    ///
    /// Only use for idempotent requests. After the last attempt a 429 becomes
    /// [`Error::RateLimited`] and other failures their usual [`Error`].
    async fn send_retrying(&self, build: impl Fn() -> RequestBuilder) -> Result<Response, Error> {
        let mut retry = 0u32;
        loop {
            let last = retry + 1 >= self.retry.max_attempts;
            match build().send().await {
                Ok(resp) => {
                    let s = resp.status();
                    if (s == StatusCode::TOO_MANY_REQUESTS || s.is_server_error()) && !last {
                        let wait = self.retry.delay(retry, retry_after_secs(resp.headers()));
                        tokio::time::sleep(wait).await;
                        retry += 1;
                        continue;
                    }
                    return Error::check(resp).await;
                }
                Err(e) if (e.is_connect() || e.is_timeout()) && !last => {
                    tokio::time::sleep(self.retry.delay(retry, None)).await;
                    retry += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Send `method` to `url` and expect an empty success body (e.g. 204).
    /// Retried, so only use it for idempotent methods.
    pub async fn send_no_content(&self, method: reqwest::Method, url: String) -> Result<(), Error> {
        self.send_retrying(|| self.client.request(method.clone(), &url))
            .await?;
        Ok(())
    }

    /// GET `url`, map non-2xx to [`Error`], decode the JSON body.
    pub async fn get_json<T: DeserializeOwned>(
        &self,
        url: String,
        params: &[(&str, String)],
    ) -> Result<T, Error> {
        self.get_json_timeout(url, params, None).await
    }

    /// Like [`get_json`](Self::get_json) with a per-request timeout override (large downloads).
    pub async fn get_json_timeout<T: DeserializeOwned>(
        &self,
        url: String,
        params: &[(&str, String)],
        timeout: Option<Duration>,
    ) -> Result<T, Error> {
        let resp = self
            .send_retrying(|| {
                let mut r = self.client.get(&url).query(params);
                if let Some(t) = timeout {
                    r = r.timeout(t);
                }
                r
            })
            .await?;
        Ok(resp.json().await?)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    /// Serves `responses` in order (the last one repeats). Returns the base URL and a request counter.
    async fn mock(responses: Vec<&'static str>) -> (String, Arc<AtomicUsize>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}", listener.local_addr().unwrap());
        let count = Arc::new(AtomicUsize::new(0));
        let c = count.clone();
        tokio::spawn(async move {
            loop {
                let (mut sock, _) = listener.accept().await.unwrap();
                let i = c.fetch_add(1, Ordering::SeqCst).min(responses.len() - 1);
                let mut buf = [0u8; 4096];
                let _ = sock.read(&mut buf).await;
                let _ = sock.write_all(responses[i].as_bytes()).await;
                let _ = sock.shutdown().await;
            }
        });
        (url, count)
    }

    const OK: &str = "HTTP/1.1 200 OK\r\nContent-Length: 3\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n[1]";
    const E503: &str =
        "HTTP/1.1 503 Service Unavailable\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const E429: &str = "HTTP/1.1 429 Too Many Requests\r\nRetry-After: 0\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";
    const E404: &str = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\nConnection: close\r\n\r\n";

    fn fast() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(1),
        }
    }

    #[test]
    fn backoff_doubles_and_honours_retry_after() {
        let p = RetryPolicy::default();
        assert_eq!(p.delay(0, None), Duration::from_millis(500));
        assert_eq!(p.delay(1, None), Duration::from_secs(1));
        assert_eq!(p.delay(2, None), Duration::from_secs(2));
        assert_eq!(p.delay(0, Some(7)), Duration::from_secs(7));
        assert_eq!(p.delay(0, Some(9999)), MAX_RETRY_AFTER);
    }

    #[tokio::test]
    async fn retries_5xx_then_succeeds() {
        let (url, n) = mock(vec![E503, E503, OK]).await;
        let h = HttpCore::new(&url).with_retry(fast());
        let v: Vec<u32> = h.get_json(format!("{url}/x"), &[]).await.unwrap();
        assert_eq!(v, [1]);
        assert_eq!(n.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn gives_up_with_status_or_rate_limited() {
        let (url, n) = mock(vec![E503]).await;
        let h = HttpCore::new(&url).with_retry(fast());
        let r = h.get_json::<Vec<u32>>(format!("{url}/x"), &[]).await;
        assert!(matches!(r, Err(Error::Status(s)) if s == 503), "{r:?}");
        assert_eq!(n.load(Ordering::SeqCst), 3);

        let (url, _) = mock(vec![E429]).await;
        let h = HttpCore::new(&url).with_retry(fast());
        let r = h.get_json::<Vec<u32>>(format!("{url}/x"), &[]).await;
        assert!(
            matches!(
                r,
                Err(Error::RateLimited {
                    retry_after: Some(0)
                })
            ),
            "{r:?}"
        );
    }

    #[tokio::test]
    async fn does_not_retry_404_or_when_disabled() {
        let (url, n) = mock(vec![E404]).await;
        let h = HttpCore::new(&url).with_retry(fast());
        let r = h.get_json::<Vec<u32>>(format!("{url}/x"), &[]).await;
        assert!(matches!(r, Err(Error::NotFound)));
        assert_eq!(n.load(Ordering::SeqCst), 1);

        let (url, n) = mock(vec![E503, OK]).await;
        let h = HttpCore::new(&url).with_retry(RetryPolicy::NONE);
        assert!(
            h.get_json::<Vec<u32>>(format!("{url}/x"), &[])
                .await
                .is_err()
        );
        assert_eq!(n.load(Ordering::SeqCst), 1);
    }
}
