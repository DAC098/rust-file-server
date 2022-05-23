use std::pin::Pin;
use std::time::Instant;
use std::future::Future;

use tower::{Layer, Service};
use hyper::header::ToStrError;
use lib::time::format_duration;

use crate::http::{header::copy_header_value, self};

pub struct LogLayer {}

impl LogLayer {
    pub fn new() -> LogLayer {
        LogLayer {}
    }
}

impl<S> Layer<S> for LogLayer {
    type Service = LogService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        LogService { inner }
    }
}

pub struct LogService<S> {
    inner: S
}

impl<S> Service<http::Request> for LogService<S>
where
    S: Service<
        http::Request,
        Response = http::Response,
        Error = http::error::Error,
        Future = Pin<Box<dyn Future<Output = http::error::Result<http::Response>> + Send>>
    >
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: http::Request) -> Self::Future {
        let info = if log::log_enabled!(log::Level::Info) {
            Some(RequestInfo::new(&req))
        } else {
            None
        };

        let future = self.inner.call(req);

        Box::pin(async move {
            let response = future.await;

            if let Some(info) = info {
                if let Ok(res) = response.as_ref() {
                    let duration = {
                        let d = info.start.elapsed();
                        format_duration(&d)
                    };
                    let status = {
                        let s = res.status();
                        s.as_str().to_owned()
                    };
                    let mut msg = String::new();
    
                    if info.remote_addr.is_some() && info.remote_port.is_some() {
                        let remote_addr = info.remote_addr.unwrap();
                        let remote_port = info.remote_port.unwrap();
    
                        if remote_addr.is_ok() && remote_port.is_ok() {
                            let addr = remote_addr.unwrap();
                            let port = remote_port.unwrap();
    
                            msg.reserve(addr.len() + 1 + port.len());
                            msg.push_str(&addr);
                            msg.push(':');
                            msg.push_str(&port);
                        }
                    }
    
                    msg.reserve(
                        info.method.len() +
                        info.path.len() +
                        info.query.len() +
                        info.version.len() +
                        status.len() +
                        duration.len() +
                        5
                    );
                    msg.push(' ');
                    msg.push_str(&info.method);
                    msg.push(' ');
                    msg.push_str(&info.path);
                    msg.push_str(&info.query);
                    msg.push(' ');
                    msg.push_str(&info.version);
                    msg.push(' ');
                    msg.push_str(&status);
                    msg.push(' ');
                    msg.push_str(&duration);
    
                    log::info!("{}", msg);
                }
            }

            response
        })
    }
}

struct RequestInfo {
    remote_addr: Option<std::result::Result<String, ToStrError>>,
    remote_port: Option<std::result::Result<String, ToStrError>>,
    version: String,
    method: String,
    path: String,
    query: String,
    start: Instant,
}

impl RequestInfo {
    pub fn new<ReqBody>(req: &hyper::Request<ReqBody>) -> RequestInfo {
        RequestInfo {
            remote_addr: copy_header_value(req.headers(), "x-forwarded-for"),
            remote_port: copy_header_value(req.headers(), "x-forwarded-port"),
            version: format!("{:?}", req.version()),
            method: req.method().as_str().to_owned(),
            path: req.uri().path().to_owned(),
            query: if let Some(q) = req.uri().query() {
                let mut query = String::with_capacity(q.len() + 1);
                query.push('?');
                query.push_str(q);
                query
            } else {
                String::new()
            },
            start: std::time::Instant::now()
        }
    }
}