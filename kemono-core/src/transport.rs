//! The seam between the clients and the network.
//!
//! [`HttpCore`](crate::client::HttpCore) builds a [`Request`], hands it to a
//! [`Transport`] and maps the [`Response`] to data or an [`Error`](crate::error::Error).
//! Status mapping, retries and JSON decoding live above this trait, so a transport
//! only has to move bytes. With the default `reqwest` feature,
//! [`ReqwestTransport`] is provided; for wasm or Cloudflare Workers implement
//! [`Transport`] on top of `fetch` (see the README).
//!
//! # Contract
//! - **Do not follow redirects.** `login` reads the `Location` of a 3xx.
//! - `timeout` is best-effort; runtimes without timers may ignore it.
//! - Return every response header, duplicates included (`Set-Cookie`).
//! - A transport may drop headers its platform forbids (e.g. `User-Agent` in browsers).
//! - Report connection failures and timeouts with the matching [`ErrorKind`]; only
//!   those are retried.

use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::time::Duration;

#[cfg(feature = "reqwest")]
mod via_reqwest;
#[cfg(feature = "reqwest")]
pub use self::via_reqwest::ReqwestTransport;

#[cfg(test)]
pub(crate) mod mock;

/// `Send` on native targets, nothing on wasm32 (where JS futures are `!Send`).
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSend: Send {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Send + ?Sized> MaybeSend for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSend {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSend for T {}

/// `Sync` on native targets, nothing on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub trait MaybeSync: Sync {}
#[cfg(not(target_arch = "wasm32"))]
impl<T: Sync + ?Sized> MaybeSync for T {}
#[cfg(target_arch = "wasm32")]
pub trait MaybeSync {}
#[cfg(target_arch = "wasm32")]
impl<T: ?Sized> MaybeSync for T {}

/// A boxed future, `Send` except on wasm32.
#[cfg(not(target_arch = "wasm32"))]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;
/// A boxed future, `Send` except on wasm32.
#[cfg(target_arch = "wasm32")]
pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Method {
    Get,
    Post,
    Delete,
}

impl Method {
    pub fn as_str(self) -> &'static str {
        match self {
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Delete => "DELETE",
        }
    }
}

/// An HTTP request with the query string already in `url`.
#[derive(Debug, Clone)]
pub struct Request {
    pub method: Method,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<Vec<u8>>,
    pub timeout: Option<Duration>,
}

/// An HTTP response with the whole body read.
#[derive(Debug, Clone)]
pub struct Response {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Response {
    /// First header named `name` (case-insensitive).
    pub fn header<'a>(&'a self, name: &'a str) -> Option<&'a str> {
        self.headers_all(name).next()
    }

    /// Every header named `name` (case-insensitive), in order.
    pub fn headers_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a str> {
        self.headers
            .iter()
            .filter(move |(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

/// How a transport failed; only `Connect` and `Timeout` are retried.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    Connect,
    Timeout,
    Other,
}

/// A failure to get a response at all (not an HTTP error status).
#[derive(Debug)]
pub struct TransportError {
    pub kind: ErrorKind,
    pub source: Box<dyn std::error::Error + Send + Sync>,
}

impl TransportError {
    pub fn new(
        kind: ErrorKind,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self {
            kind,
            source: source.into(),
        }
    }
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.source)
    }
}

impl std::error::Error for TransportError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&*self.source)
    }
}

/// Moves one request to the server and the response back. See the [module docs](self) for the contract.
pub trait Transport: MaybeSend + MaybeSync {
    fn send(&self, req: Request) -> BoxFuture<'_, Result<Response, TransportError>>;

    /// Waits `d` (used for retry backoff).
    fn sleep(&self, d: Duration) -> BoxFuture<'_, ()>;
}
