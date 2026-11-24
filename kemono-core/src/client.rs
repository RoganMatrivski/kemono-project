//! HTTP plumbing shared by the clients.
//!
//! [`HttpCore`] wraps a [`Transport`] and a base URL and maps HTTP statuses to
//! [`Error`] so each client only has to build paths and types. Requests made
//! through it (`GET`, and the idempotent no-content calls) are retried with backoff.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::error::{Error, retry_after_secs};
use crate::transport::{ErrorKind, Method, Request, Response, Transport};

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

/// A [`Transport`] plus a base URL, with status-to-[`Error`] mapping, retries,
/// default headers and an optional `session` cookie.
#[derive(Clone)]
pub struct HttpCore {
    transport: Arc<dyn Transport>,
    base_url: String,
    retry: RetryPolicy,
    timeout: Duration,
    headers: Vec<(String, String)>,
    session: Arc<Mutex<Option<String>>>,
}

impl std::fmt::Debug for HttpCore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpCore")
            .field("base_url", &self.base_url)
            .field("retry", &self.retry)
            .finish_non_exhaustive()
    }
}

impl HttpCore {
    /// `base_url` like `https://pawchive.pw/api`; a trailing slash is trimmed.
    pub fn new(base_url: &str, transport: Arc<dyn Transport>) -> Self {
        Self {
            transport,
            base_url: base_url.trim_end_matches('/').to_string(),
            retry: RetryPolicy::default(),
            timeout: DEFAULT_TIMEOUT,
            headers: vec![("User-Agent".into(), "kemono-rs/0.1.0".into())],
            session: Arc::default(),
        }
    }

    pub fn with_retry(mut self, retry: RetryPolicy) -> Self {
        self.retry = retry;
        self
    }

    /// Sends this header with every request (replacing an earlier one of the same name).
    pub fn with_header(mut self, name: &str, value: &str) -> Self {
        self.headers.retain(|(k, _)| !k.eq_ignore_ascii_case(name));
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The `session` cookie sent with every request, if any. Shared by all clones.
    pub fn session(&self) -> Option<String> {
        self.session.lock().unwrap().clone()
    }

    pub fn set_session(&self, session: Option<String>) {
        *self.session.lock().unwrap() = session;
    }

    fn request(
        &self,
        method: Method,
        url: &str,
        params: &[(&str, String)],
        timeout: Option<Duration>,
    ) -> Result<Request, Error> {
        let mut u = url::Url::parse(url).map_err(|_| Error::InvalidUrl(url.to_string()))?;
        if !params.is_empty() {
            u.query_pairs_mut()
                .extend_pairs(params.iter().map(|(k, v)| (*k, v.as_str())));
        }
        let mut headers = self.headers.clone();
        if let Some(s) = self.session() {
            headers.push(("Cookie".into(), format!("session={s}")));
        }
        Ok(Request {
            method,
            url: u.into(),
            headers,
            body: None,
            timeout: Some(timeout.unwrap_or(self.timeout)),
        })
    }

    /// Send `req`, retrying per the policy.
    ///
    /// Only use for idempotent requests. After the last attempt a 429 becomes
    /// [`Error::RateLimited`] and other failures their usual [`Error`].
    async fn send_retrying(&self, req: Request) -> Result<Response, Error> {
        let mut retry = 0u32;
        loop {
            let last = retry + 1 >= self.retry.max_attempts;
            match self.transport.send(req.clone()).await {
                Ok(resp) => {
                    let s = resp.status;
                    if (s == 429 || (500..600).contains(&s)) && !last {
                        let wait = self.retry.delay(retry, retry_after_secs(&resp));
                        self.transport.sleep(wait).await;
                        retry += 1;
                        continue;
                    }
                    return Error::check(resp);
                }
                Err(e) if matches!(e.kind, ErrorKind::Connect | ErrorKind::Timeout) && !last => {
                    self.transport.sleep(self.retry.delay(retry, None)).await;
                    retry += 1;
                }
                Err(e) => return Err(e.into()),
            }
        }
    }

    /// Send `method` to `url` and expect an empty success body (e.g. 204).
    /// Retried, so only use it for idempotent methods.
    pub async fn send_no_content(&self, method: Method, url: String) -> Result<(), Error> {
        let req = self.request(method, &url, &[], None)?;
        self.send_retrying(req).await?;
        Ok(())
    }

    /// POST a urlencoded form and return the raw response (any status, no retry).
    pub async fn post_form(&self, url: String, form: &[(&str, &str)]) -> Result<Response, Error> {
        let mut req = self.request(Method::Post, &url, &[], None)?;
        req.headers.push((
            "Content-Type".into(),
            "application/x-www-form-urlencoded".into(),
        ));
        req.body = Some(
            url::form_urlencoded::Serializer::new(String::new())
                .extend_pairs(form)
                .finish()
                .into_bytes(),
        );
        Ok(self.transport.send(req).await?)
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
        let req = self.request(Method::Get, &url, params, timeout)?;
        let resp = self.send_retrying(req).await?;
        Ok(serde_json::from_slice(&resp.body)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::TransportError;
    use crate::transport::mock::{MockTransport, conn_err, resp, resp_h};

    fn fast() -> RetryPolicy {
        RetryPolicy {
            max_attempts: 3,
            base_delay: Duration::from_millis(1),
        }
    }

    fn core(t: &Arc<MockTransport>) -> HttpCore {
        HttpCore::new("http://x", t.clone()).with_retry(fast())
    }

    async fn get(h: &HttpCore) -> Result<Vec<u32>, Error> {
        h.get_json("http://x/y".into(), &[]).await
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
    async fn retries_5xx_and_connect_errors_then_succeeds() {
        let t = MockTransport::scripted(vec![
            Ok(resp(503, "")),
            Err(conn_err()),
            Ok(resp(200, "[1]")),
        ]);
        assert_eq!(get(&core(&t)).await.unwrap(), [1]);
        assert_eq!(t.requests().len(), 3);
        assert_eq!(t.slept().len(), 2);
    }

    #[tokio::test]
    async fn gives_up_with_status_or_rate_limited() {
        let t = MockTransport::scripted(vec![Ok(resp(503, ""))]);
        let r = get(&core(&t)).await;
        assert!(matches!(r, Err(Error::Status(503))), "{r:?}");
        assert_eq!(t.requests().len(), 3);

        let t = MockTransport::scripted(vec![Ok(resp_h(429, &[("Retry-After", "0")], ""))]);
        let r = get(&core(&t)).await;
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
    async fn does_not_retry_404_or_when_disabled_or_other_errors() {
        let t = MockTransport::scripted(vec![Ok(resp(404, ""))]);
        assert!(matches!(get(&core(&t)).await, Err(Error::NotFound)));
        assert_eq!(t.requests().len(), 1);

        let t = MockTransport::scripted(vec![Ok(resp(503, "")), Ok(resp(200, "[1]"))]);
        let h = core(&t).with_retry(RetryPolicy::NONE);
        assert!(get(&h).await.is_err());
        assert_eq!(t.requests().len(), 1);

        let other = TransportError::new(ErrorKind::Other, "boom");
        let t = MockTransport::scripted(vec![Err(other)]);
        assert!(matches!(get(&core(&t)).await, Err(Error::Transport(_))));
        assert_eq!(t.requests().len(), 1);
    }

    #[tokio::test]
    async fn bad_json_is_a_decode_error() {
        let t = MockTransport::scripted(vec![Ok(resp(200, "nope"))]);
        assert!(matches!(get(&core(&t)).await, Err(Error::Decode(_))));
    }

    #[tokio::test]
    async fn builds_query_headers_and_session_cookie() {
        let t = MockTransport::scripted(vec![Ok(resp(200, "[]"))]);
        let h = core(&t).with_header("Accept", "text/css");
        h.set_session(Some("abc".into()));
        let _: Vec<u32> = h
            .get_json(
                "http://x/y".into(),
                &[("q", "a b&c".into()), ("o", "50".into())],
            )
            .await
            .unwrap();
        let r = &t.requests()[0];
        assert_eq!(r.url, "http://x/y?q=a+b%26c&o=50");
        let has = |k: &str, v: &str| r.headers.iter().any(|(a, b)| a == k && b == v);
        assert!(has("Cookie", "session=abc"));
        assert!(has("Accept", "text/css"));
        assert!(has("User-Agent", "kemono-rs/0.1.0"));
        assert_eq!(r.timeout, Some(DEFAULT_TIMEOUT));
    }

    #[tokio::test]
    async fn invalid_url_is_reported() {
        let t = MockTransport::scripted(vec![Ok(resp(200, "[]"))]);
        let h = HttpCore::new("nonsense", t.clone());
        let r: Result<Vec<u32>, _> = h.get_json("nonsense/y".into(), &[]).await;
        assert!(matches!(r, Err(Error::InvalidUrl(_))));
        assert!(t.requests().is_empty());
    }
}
