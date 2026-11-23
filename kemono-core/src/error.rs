//! Error type for the fork clients. Server messages come from `{"error": "..."}` bodies.

use std::fmt;

/// Errors from fork clients (currently Pawchive).
#[derive(Debug)]
pub enum Error {
    /// Transport failure, or a body that could not be decoded.
    Http(reqwest::Error),
    /// 404.
    NotFound,
    /// 400, with the server's `{"error": "..."}` message when present.
    BadRequest(String),
    /// 401: a session cookie is required.
    Unauthorized,
    /// Any other non-success status.
    Status(reqwest::StatusCode),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Http(e) => write!(f, "http error: {e}"),
            Error::NotFound => write!(f, "not found"),
            Error::BadRequest(m) => write!(f, "bad request: {m}"),
            Error::Unauthorized => write!(f, "unauthorized (login required)"),
            Error::Status(s) => write!(f, "unexpected status {s}"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Error::Http(e) => Some(e),
            _ => None,
        }
    }
}

impl From<reqwest::Error> for Error {
    fn from(e: reqwest::Error) -> Self {
        Error::Http(e)
    }
}

impl Error {
    /// Maps a response to `Ok(response)` or the matching error.
    pub(crate) async fn check(resp: reqwest::Response) -> Result<reqwest::Response, Error> {
        use reqwest::StatusCode as S;
        let status = resp.status();
        if status.is_success() {
            return Ok(resp);
        }
        Err(match status {
            S::NOT_FOUND => Error::NotFound,
            S::UNAUTHORIZED => Error::Unauthorized,
            S::BAD_REQUEST => {
                let body = resp.text().await.unwrap_or_default();
                Error::BadRequest(error_message(&body))
            }
            s => Error::Status(s),
        })
    }
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
