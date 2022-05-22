use std::task::Poll;
use std::{collections::HashMap, pin::Pin};
use std::future::Future;

use hyper::Method;
use tower::Service;

use crate::http::response::JsonResponseBuilder;
use crate::http::{
    Request,
    Response,
    error::{
        Result,
        Error
    }
};

#[derive(Clone)]
pub struct Endpoint<S> {
    map: HashMap<Method, S>
}

impl<S> Endpoint<S> {

    pub fn new() -> Endpoint<S> {
        Endpoint { map: HashMap::new() }
    }

    pub fn with<const N: usize>(list: [(Method, S); N]) -> Endpoint<S> {
        Endpoint { map: HashMap::from(list) }
    }

    pub fn add(&mut self, method: Method, service: S) -> () {
        self.map.insert(method, service);
    }
}

impl<S> Service<Request> for Endpoint<S>
where
    S: Service<
        Request,
        Response = Response,
        Error = Error,
        Future = Pin<Box<dyn Future<Output = Result<Response>> + Send>> 
    >
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<std::result::Result<(), Self::Error>> {
        for svc in self.map.values_mut() {
            match svc.poll_ready(cx) {
                Poll::Ready(res) => {
                    if res.is_err() {
                        return Poll::Ready(res)
                    }
                },
                Poll::Pending => {
                    return Poll::Pending;
                }
            }
        }

        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let method = req.method();

        if let Some(service) = self.map.get_mut(method) {
            service.call(req)
        } else {
            let res = JsonResponseBuilder::new(405)
                .set_error("MethodNotAllowed")
                .set_message("requested method is not accepted by this resource")
                .response();

            Box::pin(async move {
                res
            })
        }
    }
}