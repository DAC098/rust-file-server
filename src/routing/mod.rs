use std::convert::Infallible;
use std::error::Error as StdError;
use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};
use std::time::Instant;
use std::result::Result as StdResult;

use hyper::header::ToStrError;
use lib::time::format_duration;
use hyper::server::conn::AddrStream;
use hyper::{Response as HttpResponse, Method};
use hyper::service::Service;
use log::{log_enabled, Level};

use crate::components;
use crate::http::Request;
use crate::http::Response;
use crate::http::header::copy_header_value;
use crate::http::response::{okay_response, JsonResponseBuilder};
use crate::state::AppState;
use crate::http::error::{Error, Result};

mod handle;
// mod router;

#[allow(dead_code)]
#[inline]
fn method_not_allowed() -> Error {
    Error::new(405, "MethodNotAllowed", "requested method is not accepted by this resource")
}

#[allow(dead_code)]
#[inline]
fn not_found() -> Error {
    Error::new(404, "ResourceNotFound", "requested resource was not found")
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
    pub fn new(req: &Request) -> RequestInfo {
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

pub struct Router {
    connection: String,
    state: AppState
}

impl Router {
    async fn handle_route(state: AppState, req: Request) -> Result<Response> {
        let url = req.uri();
        let path = url.path();
        let method = req.method();

        if path.len() == 0 || path == "/" {
            return match *method {
                Method::GET => handle::handle_get(state, req).await,
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
            } else if path == "/auth/session/check" {
                return match *method {
                    Method::GET => handle::auth::session::check::handle_get(state, req).await,
                    _ => Err(method_not_allowed())
                }
            } else if path.starts_with("/auth/session/") {
                return match *method {
                    Method::GET => handle::auth::session::session_id::handle_get(state, req).await,
                    Method::DELETE => handle::auth::session::session_id::handle_delete(state, req).await,
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
                Method::GET => components::html_wrapper(state, req, handle::listeners::handle_get).await,
                Method::POST => components::auth_wrapper(state, req, handle::listeners::handle_post).await,
                Method::DELETE => components::auth_wrapper(state, req, handle::listeners::handle_delete).await,
                _ => Err(method_not_allowed())
            }
        }

        if let Some((action, item)) = path.strip_prefix("/").unwrap().split_once("/") {
            let context = item.to_owned();

            match action {
                "fs" => match *method {
                    Method::GET => components::html_wrapper_pass(state, req, context, handle::fs::handle_get).await,
                    Method::POST => components::auth_wrapper_pass(state, req, context, handle::fs::handle_post).await,
                    Method::PUT => components::auth_wrapper_pass(state, req, context, handle::fs::handle_put).await,
                    Method::DELETE => components::auth_wrapper_pass(state, req, context, handle::fs::handle_delete).await,
                    _ => Err(method_not_allowed())
                },
                "sync" => match *method {
                    Method::PUT => handle::sync::handle_put(state, req, context).await,
                    _ => Err(method_not_allowed())
                },
                _ => handle::_static_::handle_req(state, req).await
            }
        } else {
            handle::_static_::handle_req(state, req).await
        }
    }

    fn handle_error(error: Error) -> Result<Response> {
        if let Some(err) = error.source() {
            log::error!("error during response: {}", err);
        } else {
            log::info!("error response: {}", error);
        }

        JsonResponseBuilder::new(error.status_ref().clone())
            .set_error(error.name_str())
            .set_message(error.message_str())
            .response()
    }

    fn handle_fallback() -> Response {
        HttpResponse::builder()
            .status(500)
            .header("content-type", "text/plain")
            .body("server error".into())
            .unwrap()
    }
}

impl Service<Request> for Router {
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request) -> Self::Future {
        let connection = self.connection.clone();
        let state = self.state.clone();

        Box::pin(async move {
            let info = if log_enabled!(Level::Info) {
                Some(RequestInfo::new(&req))
            } else {
                None
            };

            let rtn = match Self::handle_route(state, req).await {
                Ok(res) => Ok(res),
                Err(error) => {
                    match Self::handle_error(error) {
                        Ok(err_res) => Ok(err_res),
                        Err(err) => {
                            log::error!("error creating error response {}", err);

                            Ok(Self::handle_fallback())
                        }
                    }
                }
            };

            if let Some(info) = info {
                if let Ok(res) = rtn.as_ref() {
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
                        } else {
                            msg.push_str(&connection);
                        }
                    } else {
                        msg.push_str(&connection);
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

            rtn
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
    type Response = Router;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = StdResult<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<StdResult<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, addr: &'t AddrStream) -> Self::Future {
        let remote_addr = addr.remote_addr();

        log::info!("new connection: {}", remote_addr);

        let router = Router {
            connection: remote_addr.to_string(),
            state: self.state.clone()
        };

        Box::pin(async move {
            Ok(router)
        })
    }
}