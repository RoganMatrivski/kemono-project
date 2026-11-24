//! Error type for the fork clients. Server messages come from `{"error": "..."}` bodies.

use std::fmt;

use crate::transport::{Response, TransportError};

/// Errors from the clients.
#[derive(Debug)]
pub enum Error {
    /// The request never produced a response (connection, timeout, ...).
    Transport(TransportError),
    /// The response body was not the expected JSON.
    Decode(serde_json::Error),
    /// A request URL could not be built (bad service or ID string).
    InvalidUrl(String),
    /// 404.
    NotFound,
    /// 400, with the server's `{"error": "..."}` message when present.
    BadRequest(String),
    /// 401: a session cookie is required.
    Unauthorized,
    /// 429 after all retries; `retry_after` is the server's `Retry-After` in seconds.
    RateLimited { retry_after: Option<u64> },
    /// Any other non-success status.
    Status(u16),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Transport(e) => write!(f, "transport error: {e}"),
            Error::Decode(e) => write!(f, "decode error: {e}"),
            Error::InvalidUrl(u) => write!(f, "invalid url: {u}"),
            Error::NotFound => write!(f, "not found"),
            Error::BadRequest(m) => write!(f, "bad request: {m}"),
            Error::Unauthorized => write!(f, "unauthorized (login required)"),
            Error::RateLimited {
                retry_after: Some(s),
            } => write!(f, "rate limited, retry after {s}s"),
            Error::RateLimited { retry_after: None } => write!(f, "rate limited"),
            Error::Status(s) => write!(f, "unexpected status {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Transport(e) => Some(e),
            Error::Decode(e) => Some(e),
            _ => None,
        }
    }
}

impl From<TransportError> for Error {
    fn from(e: TransportError) -> Self {
        Error::Transport(e)
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Error::Decode(e)
    }
}

impl Error {
    /// Maps a response to `Ok(response)` or the matching error.
    pub(crate) fn check(resp: Response) -> Result<Response, Error> {
        if (200..300).contains(&resp.status) {
            return Ok(resp);
        }
        Err(match resp.status {
            404 => Error::NotFound,
            401 => Error::Unauthorized,
            429 => Error::RateLimited {
                retry_after: retry_after_secs(&resp),
            },
            400 => Error::BadRequest(error_message(&String::from_utf8_lossy(&resp.body))),
            s => Error::Status(s),
        })
    }
}

/// `Retry-After` as seconds (the HTTP-date form is ignored).
pub(crate) fn retry_after_secs(resp: &Response) -> Option<u64> {
    resp.header("retry-after")?.trim().parse().ok()
}

fn error_message(body: &str) -> String {
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error")?.as_str().map(str::to_owned))
        .unwrap_or_else(|| body.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_extraction() {
        assert_eq!(
            error_message(r#"{"error":"offset not multiple of 50"}"#),
            "offset not multiple of 50"
        );
        assert_eq!(error_message("plain"), "plain");
    }
}
