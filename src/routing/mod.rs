use std::pin::Pin;
use std::future::Future;
use std::task::{Context, Poll};

use lib::time::format_duration;
use hyper::server::conn::AddrStream;
use hyper::{Request, Response, Body, Error, Method};
use hyper::service::Service;

use crate::components::html::{check_if_html, responed_index_html};
use crate::db::ArcDBState;
use crate::http::response::okay_response;
use crate::snowflakes::IdSnowflakes;
use crate::storage::ArcStorageState;
use crate::template::ArcTemplateState;
use crate::http::error::{
    Error as ResponseError,
    Result as ResponseResult
};

mod handle;

#[inline]
fn method_not_allowed() -> ResponseError {
    ResponseError {
        status: 405,
        name: "MethodNotAllowed".to_owned(),
        msg: "requested method is not accepted by this resource".to_owned(),
        source: None
    }
}

pub struct Router<'a> {
    connection: String,
    db: ArcDBState,
    storage: ArcStorageState,
    template: ArcTemplateState<'a>,
    snowflakes: IdSnowflakes,
}

impl<'a> Router<'a> {

    async fn handle_route(req: Request<Body>) -> ResponseResult<Response<Body>> {
        let url = req.uri();
        let path = url.path();
        let method = req.method();

        if path.starts_with("/auth/") {
            if path == "/auth/session" {
                return match *method {
                    Method::GET => handle::auth::session::handle_get(req).await,
                    Method::POST => handle::auth::session::handle_post(req).await,
                    Method::DELETE => handle::auth::session::handle_delete(req).await,
                    _ => Err(method_not_allowed())
                }
            } else if path == "/auth/password" {
                return match *method {
                    Method::POST => handle::auth::password::handle_post(req).await,
                    _ => Err(method_not_allowed())
                }
            }
        } else if path.starts_with("/admin/") {
            if path == "/admin/users" {
                return match *method {
                    Method::GET => okay_response(req),
                    Method::POST => okay_response(req),
                    Method::DELETE => okay_response(req),
                    _ => Err(method_not_allowed())
                }
            } else if path.starts_with("/admin/users") {
                return match *method {
                    Method::GET => okay_response(req),
                    Method::PUT => okay_response(req),
                    Method::DELETE => okay_response(req),
                    _ => Err(method_not_allowed())
                }
            }
        } else if path.starts_with("/fs/") {
            return match *method {
                Method::GET => handle::fs::handle_get(req).await,
                Method::POST => handle::fs::handle_post(req).await,
                Method::PUT => handle::fs::handle_put(req).await,
                Method::DELETE => handle::fs::handle_delete(req).await,
                _ => Err(method_not_allowed())
            }
        }

        handle::_static_::handle_req(req).await
    }

    fn handle_error(error: ResponseError) -> ResponseResult<Response<Body>> {
        if let Some(source) = error.source {
            println!("error during response: {}", source);
        }

        // this probably needs to be handled better
        Response::builder()
            .status(error.status)
            .header("content-type", "application/json")
            .body(format!("{{\"error\":\"{}\",\"message\":\"{}\"}}", error.name, error.msg).into())
            .map_err(|err| err.into())
    }

    fn handle_fallback() -> Response<Body> {
        Response::builder()
            .status(500)
            .header("content-type", "text/plain")
            .body("server error".into())
            .unwrap()
    }
}

impl Service<Request<Body>> for Router<'static> {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let connection = self.connection.clone();
        let extensions_ref = req.extensions_mut();
        extensions_ref.insert(self.db.clone());
        extensions_ref.insert(self.storage.clone());
        extensions_ref.insert(self.template.clone());
        extensions_ref.insert(self.snowflakes.clone());

        Box::pin(async move {
            let version = format!("{:?}", req.version());
            let method = req.method().as_str().to_owned();
            let path = req.uri().path().to_owned();
            let query = if let Some(q) = req.uri().query() {
                format!("?{}", q)
            } else {
                String::new()
            };

            let start = std::time::Instant::now();

            let rtn = match Self::handle_route(req).await {
                Ok(res) => Ok(res),
                Err(error) => {
                    match Self::handle_error(error) {
                        Ok(err_res) => Ok(err_res),
                        Err(err) => {
                            println!("error creating error response {}", err);

                            Ok(Self::handle_fallback())
                        }
                    }
                }
            };

            if let Ok(res) = rtn.as_ref() {
                let duration = start.elapsed();

                println!(
                    "{} {} {}{} {} {} {}",
                    connection,
                    method,
                    path, query,
                    version,
                    res.status(),
                    format_duration(&duration)
                );
            }

            rtn
        })
    }
}

#[derive(Clone)]
pub struct MakeRouter<'a> {
    pub db: ArcDBState,
    pub storage: ArcStorageState,
    pub template: ArcTemplateState<'a>,
    pub snowflakes: IdSnowflakes,
}

// so for how this works. the service works in two steps. the first service 
// accepts the target of the inbound connection. from there, this service 
// will return another that will work on any requests from that connection.
impl<'t> Service<&'t AddrStream> for MakeRouter<'static> {
    type Response = Router<'static>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, addr: &'t AddrStream) -> Self::Future {
        let remote_addr = addr.remote_addr();

        println!("new connection: {}", remote_addr);

        let router = Router {
            connection: remote_addr.to_string(),
            db: self.db.clone(),
            storage: self.storage.clone(),
            template: self.template.clone(),
            snowflakes: self.snowflakes.clone(),
        };

        Box::pin(async move {
            Ok(router)
        })
    }
}