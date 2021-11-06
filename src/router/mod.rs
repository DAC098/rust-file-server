use std::pin::{Pin};
use std::future::{Future};
use std::task::{Context, Poll};

use hyper::{Request, Response, Body, Error};
use hyper::service::{Service};

use regex::{Regex};

use crate::db::shared_state::{ArcDBState};
use crate::storage::shared_state::{ArcStorageState};

mod handle;

lazy_static::lazy_static! {
    static ref FS_PATH: Regex = Regex::new("/fs/").unwrap();
}

fn make_plain_text_response(msg: &'static str) -> Response<Body> {
    Response::builder()
        .status(200)
        .header("Content-Type", "text/plain")
        .body(Body::from(msg))
        .unwrap()
}

pub struct Router {
    db: ArcDBState,
    storage: ArcStorageState
}

impl Service<Request<Body>> for Router {
    type Response = Response<Body>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, mut req: Request<Body>) -> Self::Future {
        let extensions_ref = req.extensions_mut();
        extensions_ref.insert(self.db.clone());
        extensions_ref.insert(self.storage.clone());

        Box::pin(async move {
            let url = req.uri();

            if url.path().starts_with("/fs") {
                handle::fs::handle_get(req).await
            } else {
                Ok(make_plain_text_response("not found"))
            }
        })
    }
}

#[derive(Clone)]
pub struct MakeRouter {
    pub db: ArcDBState,
    pub storage: ArcStorageState
}

// okay so for a reason that I still do not understand, this will only work if the Service
// target is a generic and the future implements the Send trait as well.
// otherwise the error that will happen from the compiler will start talking about
// this particular snippet failing
//
// error[E0277]: the trait bound `MakeRouter: hyper::service::make::MakeServiceRef<AddrStream, Body>` is not satisfied
//   --> src/main.rs:99:47
//    |
// 99 |     if let Err(e) = Server::bind(&addr).serve(svc).await {
//    |                                               ^^^
//    |                                               |
//    |                                               expected an implementor of trait `hyper::service::make::MakeServiceRef<AddrStream, Body>`
//    |                                               help: consider mutably borrowing here: `&mut svc`
//    |
//    = note: required because of the requirements on the impl of `hyper::service::make::MakeServiceRef<AddrStream, Body>` for `MakeRouter`

// error[E0277]: the trait bound `for<'a> MakeRouter: Service<&'a AddrStream>` is not satisfied
//   --> src/main.rs:99:21
//    |
// 99 |     if let Err(e) = Server::bind(&addr).serve(svc).await {
//    |                     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^ the trait `for<'a> Service<&'a AddrStream>` is not implemented for `MakeRouter`
//    |
//    = help: the following implementations were found:
//              <MakeRouter as Service<&'t AddrIncoming>>
//    = note: required because of the requirements on the impl of `hyper::service::make::MakeServiceRef<AddrStream, Body>` for `MakeRouter`
//    = note: required because of the requirements on the impl of `futures::Future` for `Server<AddrIncoming, MakeRouter>`
// note: required by `futures::Future::poll`
//   --> /home/dac098/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/src/rust/library/core/src/future/future.rs:99:5
//    |
// 99 |     fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output>;
//    |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

// error[E0277]: the trait bound `for<'a> MakeRouter: Service<&'a AddrStream>` is not satisfied
//   --> src/main.rs:99:41
//    |
// 99 |     if let Err(e) = Server::bind(&addr).serve(svc).await {
//    |                                         ^^^^^ the trait `for<'a> Service<&'a AddrStream>` is not implemented for `MakeRouter`
//    |
//    = help: the following implementations were found:
//              <MakeRouter as Service<&'t AddrIncoming>>
//    = note: required because of the requirements on the impl of `hyper::service::make::MakeServiceRef<AddrStream, Body>` for `MakeRouter`

// the examples that I have been looking at never really showed working with 
// the incoming connection so when I tried to just specify that in the service
// target, the compiler complained about the above errors and still currently
// do not full understand why. something about the lifetime of the 
// &AddrIncoming that was failing to be met

// so for how this works. the service works in two steps. the first service 
// accepts the target of the inbound connection. from there, this service 
// will return another that will work on any requests from that connection.
impl<T> Service<T> for MakeRouter {
    type Response = Router;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let router = Router {
            db: self.db.clone(),
            storage: self.storage.clone()
        };

        Box::pin(async move {
            Ok(router)
        })
    }
}