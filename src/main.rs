
use std::path::{PathBuf};

use hyper::{Server};
use futures::future::{try_join_all};

mod error;

mod config;
mod db;
mod watcher;

mod router;

type JoinHandleList = Vec<tokio::task::JoinHandle<error::Result<()>>>;

fn main() {
    std::process::exit(match main_entry() {
        Ok(code) => code,
        Err(_err) => 1
    })
}

fn main_entry() -> error::Result<i32> {
    std::env::set_var("RUST_BACKTRACE", "1");

    let mut config_files: Vec<PathBuf> = Vec::new();
    let mut args = std::env::args();
    args.next();

    loop {
        let arg = match args.next() {
            Some(s) => s,
            None => break
        };

        config_files.push(
            config::get_config_file(&arg)?
        );
    }

    let conf = config::load_server_config(config_files)?;
    
    tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(conf.threads)
        // .on_thread_park(|| {
        //     println!("thread parked");
        // })
        // .on_thread_start(|| {
        //     println!("thread started");
        // })
        // .on_thread_stop(|| {
        //     println!("thread stopped");
        // })
        // .on_thread_unpark(|| {
        //     println!("thread unparked");
        // })
        .build()?
        .block_on(main_runtime(conf))
}

async fn main_runtime(conf: config::ServerConfig) -> error::Result<i32> {
    let db_conf = conf.db;
    let router = router::MakeRouter {
        db: db::build_shared_state(db::build_config(db_conf)).await?
    };
    let watch_directory = conf.directory.clone();
    let mut futures_list = JoinHandleList::new();
    futures_list.push(tokio::spawn(
        make_watcher(watch_directory)
    ));

    println!("building servers");

    for bind in conf.bind {
        let addr = bind.to_sockaddr();

        if addr.is_err() {
            print!("{}", addr.unwrap_err());
        } else {
            futures_list.push(tokio::spawn(
                make_server(addr.unwrap(), router.clone())
            ));
        }
    }

    println!("blocking on join handles");

    try_join_all(futures_list).await.unwrap();

    Ok(0)
}

async fn make_server(
    addr: std::net::SocketAddr, 
    router: router::MakeRouter,
) -> error::Result<()> {
    println!("building service stack");
    let svc = tower::ServiceBuilder::new()
        .service(router);

    println!("creating server. binding to: {}", addr);

    if let Err(e) = Server::bind(&addr).serve(svc).await {
        println!("server error. {:?}", e);
    }

    println!("server closed");

    Ok(())
}

async fn make_watcher(directory: PathBuf) -> error::Result<()> {
    println!("starting watcher");

    if let Err(e) = watcher::watch(directory).await {
        println!("error from watcher: {:?}", e);
    }

    Ok(())
}










/*
use hyper::service::Service;
use hyper::{Body, Request, Response, Server};

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

type Counter = i32;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let addr = ([127, 0, 0, 1], 3000).into();

    let server = Server::bind(&addr).serve(MakeSvc { counter: 81818 });
    println!("Listening on http://{}", addr);

    server.await?;
    Ok(())
}

struct Svc {
    counter: Counter,
}

impl Service<Request<Body>> for Svc {
    type Response = Response<Body>;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: Request<Body>) -> Self::Future {
        fn mk_response(s: String) -> Result<Response<Body>, hyper::Error> {
            Ok(Response::builder().body(Body::from(s)).unwrap())
        }

        let res = match req.uri().path() {
            "/" => mk_response(format!("home! counter = {:?}", self.counter)),
            "/posts" => mk_response(format!("posts, of course! counter = {:?}", self.counter)),
            "/authors" => mk_response(format!(
                "authors extraordinare! counter = {:?}",
                self.counter
            )),
            // Return the 404 Not Found for other routes, and don't increment counter.
            _ => return Box::pin(async { mk_response("oh no! not found".into()) }),
        };

        if req.uri().path() != "/favicon.ico" {
            self.counter += 1;
        }

        Box::pin(async { res })
    }
}

struct MakeSvc {
    counter: Counter,
}

impl<T> Service<T> for MakeSvc {
    type Response = Svc;
    type Error = hyper::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, _: &mut Context) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _: T) -> Self::Future {
        let counter = self.counter.clone();
        let fut = async move { Ok(Svc { counter }) };
        Box::pin(fut)
    }
}
*/