use std::{net::IpAddr, pin::Pin, future::Future};

use tower::{Layer, Service};

use crate::{state::AppState, http::{Request, Response, error::{Error, Result}}};

pub struct StateLayer {
    state: AppState,
    conn: IpAddr
}

impl StateLayer {
    pub fn new(state: AppState, conn: IpAddr) -> StateLayer {
        StateLayer {state, conn}
    }
}

impl<S> Layer<S> for StateLayer {
    type Service = StateService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        StateService {
            inner,
            state: self.state.clone(),
            conn: self.conn.clone()
        }
    }
}

pub struct StateService<S> {
    inner: S,
    state: AppState,
    conn: IpAddr
}

impl<S> Service<Request> for StateService<S>
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
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let extension = req.extensions_mut();
        extension.insert(self.state.clone());
        extension.insert(self.conn.clone());

        self.inner.call(req)
    }
}