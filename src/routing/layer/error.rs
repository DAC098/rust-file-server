use std::pin::Pin;
use std::future::Future;
use std::error::Error as _;

use tower::{Layer, Service};

use crate::http::{self, response::JsonResponseBuilder};

pub struct ErrorLayer {}

impl ErrorLayer {
    pub fn new() -> ErrorLayer {
        ErrorLayer {}
    }
}

impl<S> Layer<S> for ErrorLayer {
    type Service = ErrorService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        ErrorService {
            inner
        }
    }
}

pub struct ErrorService<S> {
    inner: S
}

impl<S> Service<http::Request> for ErrorService<S>
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
        let fut = self.inner.call(req);

        Box::pin(async move {
            let result = fut.await;

            if let Err(error) = result.as_ref() {
                if let Some(err) = error.source() {
                    log::error!("error during response: {}", err);
                } else {
                    log::info!("error response: {}", error);
                }

                JsonResponseBuilder::new(error.status_ref().clone())
                    .set_error(error.name_str())
                    .set_message(error.message_str())
                    .response()
            } else {
                result
            }
        })
    }
}