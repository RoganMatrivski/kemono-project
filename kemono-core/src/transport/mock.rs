//! In-memory transport for tests: scripted or computed responses, recorded requests, instant sleep.

use std::sync::Mutex;
use std::time::Duration;

use super::{BoxFuture, ErrorKind, Request, Response, Transport, TransportError};

type Handler = Box<dyn Fn(&Request, usize) -> Result<Response, TransportError> + Send + Sync>;

pub(crate) struct MockTransport {
    handler: Handler,
    log: Mutex<Vec<Request>>,
    slept: Mutex<Vec<Duration>>,
}

pub(crate) fn resp(status: u16, body: &str) -> Response {
    Response {
        status,
        headers: vec![],
        body: body.as_bytes().to_vec(),
    }
}

pub(crate) fn resp_h(status: u16, headers: &[(&str, &str)], body: &str) -> Response {
    Response {
        headers: headers
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect(),
        ..resp(status, body)
    }
}

pub(crate) fn conn_err() -> TransportError {
    TransportError::new(ErrorKind::Connect, "refused")
}

impl MockTransport {
    /// `handler(request, n)` where `n` is the 0-based request count.
    pub fn new(
        handler: impl Fn(&Request, usize) -> Result<Response, TransportError> + Send + Sync + 'static,
    ) -> std::sync::Arc<Self> {
        std::sync::Arc::new(Self {
            handler: Box::new(handler),
            log: Mutex::new(vec![]),
            slept: Mutex::new(vec![]),
        })
    }

    /// Answers with `script` in order; the last entry repeats.
    pub fn scripted(script: Vec<Result<Response, TransportError>>) -> std::sync::Arc<Self> {
        Self::new(move |_, n| match &script[n.min(script.len() - 1)] {
            Ok(r) => Ok(r.clone()),
            Err(e) => Err(TransportError::new(e.kind, e.source.to_string())),
        })
    }

    pub fn requests(&self) -> Vec<Request> {
        self.log.lock().unwrap().clone()
    }

    pub fn slept(&self) -> Vec<Duration> {
        self.slept.lock().unwrap().clone()
    }
}

impl Transport for MockTransport {
    fn send(&self, req: Request) -> BoxFuture<'_, Result<Response, TransportError>> {
        let n = {
            let mut log = self.log.lock().unwrap();
            log.push(req.clone());
            log.len() - 1
        };
        let r = (self.handler)(&req, n);
        Box::pin(std::future::ready(r))
    }

    fn sleep(&self, d: Duration) -> BoxFuture<'_, ()> {
        self.slept.lock().unwrap().push(d);
        Box::pin(std::future::ready(()))
    }
}
