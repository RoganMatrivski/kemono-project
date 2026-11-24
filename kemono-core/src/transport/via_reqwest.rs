//! [`Transport`] on `reqwest` + `tokio` (native targets).

use std::time::Duration;

use super::{BoxFuture, ErrorKind, Method, Request, Response, Transport, TransportError};

/// Sends requests with a `reqwest::Client`.
#[derive(Debug, Clone)]
pub struct ReqwestTransport {
    client: reqwest::Client,
}

impl Default for ReqwestTransport {
    fn default() -> Self {
        Self::new()
    }
}

impl ReqwestTransport {
    /// gzip on, redirects off.
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .gzip(true)
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap();
        Self { client }
    }

    /// Use a pre-configured client (proxy, TLS, ...). It must not follow redirects,
    /// or `login` cannot see the redirect.
    pub fn from_client(client: reqwest::Client) -> Self {
        Self { client }
    }
}

fn map_err(e: reqwest::Error) -> TransportError {
    let kind = if e.is_timeout() {
        ErrorKind::Timeout
    } else if e.is_connect() {
        ErrorKind::Connect
    } else {
        ErrorKind::Other
    };
    TransportError::new(kind, e)
}

impl Transport for ReqwestTransport {
    fn send(&self, req: Request) -> BoxFuture<'_, Result<Response, TransportError>> {
        Box::pin(async move {
            let method = match req.method {
                Method::Get => reqwest::Method::GET,
                Method::Post => reqwest::Method::POST,
                Method::Delete => reqwest::Method::DELETE,
            };
            let mut b = self.client.request(method, &req.url);
            for (k, v) in &req.headers {
                b = b.header(k, v);
            }
            if let Some(body) = req.body {
                b = b.body(body);
            }
            if let Some(t) = req.timeout {
                b = b.timeout(t);
            }
            let resp = b.send().await.map_err(map_err)?;
            let status = resp.status().as_u16();
            let headers = resp
                .headers()
                .iter()
                .map(|(k, v)| {
                    (
                        k.as_str().to_string(),
                        String::from_utf8_lossy(v.as_bytes()).into_owned(),
                    )
                })
                .collect();
            let body = resp.bytes().await.map_err(map_err)?.to_vec();
            Ok(Response {
                status,
                headers,
                body,
            })
        })
    }

    fn sleep(&self, d: Duration) -> BoxFuture<'_, ()> {
        Box::pin(tokio::time::sleep(d))
    }
}

#[cfg(test)]
mod tests {
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::*;

    #[tokio::test]
    async fn round_trip_without_following_redirects() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/x", listener.local_addr().unwrap());
        tokio::spawn(async move {
            let (mut s, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 4096];
            let n = s.read(&mut buf).await.unwrap();
            assert!(String::from_utf8_lossy(&buf[..n]).starts_with("GET /x?a=1 "));
            s.write_all(
                b"HTTP/1.1 303 See Other\r\nLocation: /y\r\nSet-Cookie: a=1\r\nSet-Cookie: b=2\r\nContent-Length: 2\r\nConnection: close\r\n\r\nhi",
            )
            .await
            .unwrap();
        });
        let r = ReqwestTransport::new()
            .send(Request {
                method: Method::Get,
                url: format!("{url}?a=1"),
                headers: vec![("X-Test".into(), "1".into())],
                body: None,
                timeout: Some(Duration::from_secs(5)),
            })
            .await
            .unwrap();
        assert_eq!(r.status, 303);
        assert_eq!(r.header("location"), Some("/y"));
        assert_eq!(r.headers_all("set-cookie").count(), 2);
        assert_eq!(r.body, b"hi");
    }

    #[tokio::test]
    async fn connection_refused_is_connect_error() {
        let l = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/", l.local_addr().unwrap());
        drop(l);
        let e = ReqwestTransport::new()
            .send(Request {
                method: Method::Get,
                url,
                headers: vec![],
                body: None,
                timeout: None,
            })
            .await
            .unwrap_err();
        assert_eq!(e.kind, ErrorKind::Connect);
    }
}
