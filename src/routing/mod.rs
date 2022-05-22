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

mod params;
// mod service;
mod layer;
mod handle;

#[inline]
fn method_not_allowed() -> Error {
    Error::new(405, "MethodNotAllowed", "requested method is not accepted by this resource")
}

fn join_iter<'a>(iter: &mut impl Iterator<Item = &'a str>) -> String {
    let mut rtn = String::new();
    let mut first = true;

    for seg in iter {
        if first {
            first = false;

            rtn.push_str(seg);
        } else {
            rtn.reserve(seg.len() + 1);
            rtn.push('/');
            rtn.push_str(seg)
        }
    }

    rtn
}

pub struct Router {
    connection: IpAddr,
    state: AppState
}

impl Router {
    async fn handle_route(state: AppState, mut req: Request) -> Result<Response> {
        let method = req.method().clone();
        let path = req.uri().path().to_owned();
        let segments: Vec<&str> = path.strip_prefix("/").unwrap().split("/").collect();
        let total_segments = segments.len();
        let mut segments_iter = segments.into_iter();

        if let Some(first_seg) = segments_iter.next() {
            if first_seg == "ping" && total_segments == 1 {
                return match method {
                    Method::GET => handle::ping::handle_get(req).await,
                    _ => Err(method_not_allowed())
                }
            } else if first_seg == "users" {
                if total_segments == 1 {
                    return match method {
                        Method::GET => okay_response(req),
                        Method::POST => handle::admin::users::handle_post(state, req).await,
                        Method::DELETE => okay_response(req),
                        _ => Err(method_not_allowed())
                    }
                }

                let mut params = params::Params::with([
                    ("users_id".into(), segments_iter.next().unwrap().into())
                ]);
                
                if total_segments == 2 {
                    req.extensions_mut().insert(params);

                    return match method {
                        Method::GET => okay_response(req),
                        Method::PUT => okay_response(req),
                        Method::DELETE => okay_response(req),
                        _ => Err(method_not_allowed())
                    }
                } else if let Some(third_seg) = segments_iter.next() {
                    params.insert("context", join_iter(&mut segments_iter));

                    req.extensions_mut().insert(params);

                    if third_seg == "fs" {
                        return match method {
                            Method::GET => handle::fs::handle_get(state, req).await,
                            Method::POST => handle::fs::handle_post(state, req).await,
                            Method::PUT => handle::fs::handle_put(state, req).await,
                            Method::DELETE => handle::fs::handle_delete(state, req).await,
                            _ => Err(method_not_allowed())
                        }
                    } else if third_seg == "sync" {
                        return match method {
                            Method::PUT => handle::sync::handle_put(state, req).await,
                            _ => Err(method_not_allowed())
                        }
                    }
                }
            } else if first_seg == "listeners" {
                return match method {
                    Method::GET => handle::listeners::handle_get(state, req).await,
                    Method::POST => handle::listeners::handle_post(state, req).await,
                    Method::DELETE => handle::listeners::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if first_seg == "session" {
                if total_segments == 1 {
                    return match method {
                        Method::GET => handle::session::handle_get(state, req).await,
                        Method::DELETE => handle::session::handle_delete(state, req).await,
                        _ => Err(method_not_allowed())
                    }
                }

                req.extensions_mut().insert(params::Params::with([
                    ("session_id".into(), segments_iter.next().unwrap().into())
                ]));

                return match method {
                    Method::DELETE => handle::session::session_id::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if first_seg == "fs" {
                req.extensions_mut().insert(params::Params::with([
                    ("context".into(), join_iter(&mut segments_iter))
                ]));

                return match method {
                    Method::GET => handle::fs::handle_get(state, req).await,
                    Method::POST => handle::fs::handle_post(state, req).await,
                    Method::PUT => handle::fs::handle_put(state, req).await,
                    Method::DELETE => handle::fs::handle_delete(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if first_seg == "sync" {
                req.extensions_mut().insert(params::Params::with([
                    ("context".into(), join_iter(&mut segments_iter))
                ]));

                return match method {
                    Method::PUT => handle::sync::handle_put(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if first_seg == "auth" {
                if let Some(second_seg) = segments_iter.next() {
                    if second_seg == "session" {
                        return match method {
                            Method::GET => handle::auth::session::handle_get(state, req).await,
                            Method::POST => handle::auth::session::handle_post(state, req).await,
                            Method::DELETE => handle::auth::session::handle_delete(state, req).await,
                            _ => Err(method_not_allowed())
                        }
                    } else if second_seg == "password" {
                        return match method {
                            Method::POST => handle::auth::password::handle_post(state, req).await,
                            _ => Err(method_not_allowed())
                        }
                    }
                }
            }

            handle::_static_::handle_req(state, req).await
        } else {
            match method {
                Method::GET => handle::handle_get(req).await,
                _ => Err(method_not_allowed())
            }
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