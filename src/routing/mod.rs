use std::convert::Infallible;
use std::net::IpAddr;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use std::result::Result as StdResult;

use hyper::server::conn::AddrStream;
use hyper::Method;
use hyper::service::Service;
use tower::ServiceBuilder;

use crate::http::Request;
use crate::http::Response;
use crate::http::response::okay_response;
use crate::http::header::copy_header_value;
use crate::state::AppState;
use crate::http::error::{Error, Result};

mod layer;
// mod router;
// mod endpoint;
mod handle;

#[inline]
fn method_not_allowed() -> Error {
    Error::new(405, "MethodNotAllowed", "requested method is not accepted by this resource")
}

pub struct Router {
    connection: IpAddr,
    state: AppState
}

impl Router {
    async fn handle_route(state: AppState, req: Request) -> Result<Response> {
        let path = req.uri().path();
        let method = req.method();

        if path.len() == 0 || path == "/" {
            return match *method {
                Method::GET => handle::handle_get(req).await,
                _ => Err(method_not_allowed())
            }
        } else if path == "/ping" {
            return match *method {
                Method::GET => handle::ping::handle_get(req).await,
                _ => Err(method_not_allowed())
            }
        } else if path.starts_with("/auth/") {
            if path == "/auth/session" {
                return match *method {
                    Method::GET => handle::auth::session::handle_get(state, req).await,
                    Method::POST => handle::auth::session::handle_post(state, req).await,
                    Method::DELETE => handle::auth::session::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if path == "/auth/password" {
                return match *method {
                    Method::POST => handle::auth::password::handle_post(state, req).await,
                    _ => Err(method_not_allowed())
                }
            }
        } else if path.starts_with("/admin/") {
            if path == "/admin/users" {
                return match *method {
                    Method::GET => okay_response(req),
                    Method::POST => handle::admin::users::handle_post(state, req).await,
                    Method::DELETE => okay_response(req),
                    _ => Err(method_not_allowed())
                }
            } else if path.starts_with("/admin/users/") {
                return match *method {
                    Method::GET => okay_response(req),
                    Method::PUT => okay_response(req),
                    Method::DELETE => okay_response(req),
                    _ => Err(method_not_allowed())
                }
            }
        } else if path == "/listeners" {
            return match *method {
                Method::GET => handle::listeners::handle_get(state, req).await,
                Method::POST => handle::listeners::handle_post(state, req).await,
                Method::DELETE => handle::listeners::handle_delete(state, req).await,
                _ => Err(method_not_allowed())
            }
        } else if path.starts_with("/session") {
            if path == "/session" {
                return match *method {
                    Method::GET => handle::session::handle_get(state, req).await,
                    Method::DELETE => handle::session::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if path.starts_with("/session/") {
                return match *method {
                    Method::DELETE => handle::session::session_id::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            }
        }

        if let Some((action, item)) = path.strip_prefix("/").unwrap().split_once("/") {
            match action {
                "fs" => match *method {
                    Method::GET => handle::fs::handle_get(state, req).await,
                    Method::POST => handle::fs::handle_post(state, req).await,
                    Method::PUT => handle::fs::handle_put(state, req).await,
                    Method::DELETE => handle::fs::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                },
                "sync" => match *method {
                    Method::PUT => handle::sync::handle_put(state, req).await,
                    _ => Err(method_not_allowed())
                },
                _ => handle::_static_::handle_req(state, req).await
            }
        } else {
            handle::_static_::handle_req(state, req).await
        }
    }
}

impl Service<Request> for Router {
    type Response = Response;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request) -> Self::Future {
        let mut connection = self.connection.clone();
        let state = self.state.clone();

        Box::pin(async move {
            if let Some(ip_header) = copy_header_value(req.headers(), "x-forwarded-for") {
                let ip_str = ip_header.map_err(|v| Error::from(v))?;
                
                if let Ok(ip) = ip_str.parse() {
                    connection = ip;
                }
            }

            req.extensions_mut().insert(connection);

            Self::handle_route(state, req).await
        })
    }
}

#[derive(Clone)]
pub struct MakeRouter {
    pub state: AppState
}

// so for how this works. the service works in two steps. the first service 
// accepts the target of the inbound connection. from there, this service 
// will return another that will work on any requests from that connection.
impl<'t> Service<&'t AddrStream> for MakeRouter {
    type Response = layer::LogService<layer::ErrorService<Router>>;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, addr: &'t AddrStream) -> Self::Future {
        let remote_addr = addr.remote_addr();

        let router = Router {
            connection: remote_addr.ip(),
            state: self.state.clone()
        };

        let svc = ServiceBuilder::new()
            .layer(layer::LogLayer::new())
            .layer(layer::ErrorLayer::new())
            .service(router);

        Box::pin(async move {
            Ok(svc)
        })
    }
}