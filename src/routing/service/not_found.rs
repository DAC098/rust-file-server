use std::task::Context;
use std::{task::Poll, pin::Pin};
use std::future::Future;
use std::result::Result as StdResult;

use tower::Service;

use crate::http::response::JsonResponseBuilder;
use crate::http::{Request, Response, error::Error};

#[derive(Clone)]
pub struct NotFound;

impl Service<Request> for NotFound {
    type Response = Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        Box::pin(async move {
            JsonResponseBuilder::new(404)
                .set_error("NotFound")
                .set_message("requested resource was not found")
                .response()
        })
    }
}