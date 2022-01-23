
use std::path::PathBuf;

use hyper::Server;
use futures::future::try_join_all;

mod error;

mod config;
mod db;
mod storage;
mod template;
mod watcher;
mod snowflakes;
mod security;

mod http;

mod components;

mod routing;

type JoinHandleList = Vec<tokio::task::JoinHandle<error::Result<()>>>;

fn main() {
    std::process::exit(match main_entry() {
        Ok(code) => code,
        Err(err) => {
            println!("{}", err);

            1
        }
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

    println!("{:#?}", conf);

    tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(conf.threads)
        .build()?
        .block_on(main_runtime(conf))
}

async fn main_runtime(conf: config::ServerConfig) -> error::Result<i32> {
    let db_conf = conf.db;
    let storage_conf = conf.storage;
    let template_conf = conf.template;
    let watch_directory = storage_conf.directory.clone();
    let router = routing::MakeRouter {
        db: db::DBState::new(db::build_config(db_conf)).await?,
        storage: storage_conf.into(),
        template: template::TemplateState::new(template::build_registry(template_conf)?),
        snowflakes: snowflakes::IdSnowflakes::new(1)?
    };

    let mut futures_list = JoinHandleList::new();
    futures_list.push(tokio::spawn(
        make_watcher(watch_directory)
    ));

    for bind in conf.bind {
        let addr = bind.to_sockaddr();

        if addr.is_err() {
            println!("{}", addr.unwrap_err());
        } else {
            futures_list.push(tokio::spawn(
                make_server(addr.unwrap(), router.clone())
            ));
        }
    }

    try_join_all(futures_list).await.unwrap();

    Ok(0)
}

async fn make_server(
    addr: std::net::SocketAddr, 
    router: routing::MakeRouter<'static>,
) -> error::Result<()> {
    let svc = tower::ServiceBuilder::new()
        .service(router);

    match Server::try_bind(&addr) {
        Ok(builder) => {
            println!("server listening on {}", addr);

            if let Err(e) = builder.serve(svc).await {
                println!("server error. {:?}", e);
            }
        },
        Err(err) => {
            println!("failed to bind to address. {:?}", err);
        }
    }

    Ok(())
}

async fn make_watcher(directory: PathBuf) -> error::Result<()> {
    println!("starting watcher");

    if let Err(e) = watcher::watch(directory).await {
        println!("error from watcher: {:?}", e);
    }

    Ok(())
}