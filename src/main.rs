
use std::path::PathBuf;

use hyper::Server;
use futures::future::try_join_all;
use log::{log_enabled, Level};
use tokio::runtime::Handle;

mod error;

mod config;
mod http;

mod db;
mod storage;
mod template;
mod snowflakes;
mod security;
mod state;
mod event;

mod components;

mod routing;

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

    env_logger::init();

    let conf = config::load_server_config(config_files)?;

    if log_enabled!(Level::Debug) {
        log::debug!("env vars");

        for (key, value) in std::env::vars() {
            log::debug!("{} {}", key, value);
        }
    }

    log::debug!("{:#?}", conf);

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_io()
        .enable_time()
        .worker_threads(conf.threads)
        .build()?;

    rt.block_on(main_runtime(conf, rt.handle().clone()))
}

async fn main_runtime(conf: config::ServerConfig, rt_handle: Handle) -> error::Result<i32> {
    let db_conf = conf.db;
    let storage_conf = conf.storage;
    let template_conf = conf.template;
    let state = state::AppState {
        db: db::DBState::new(db::build_config(db_conf)).await?,
        storage: storage_conf.into(),
        template: template::TemplateState::new(template::build_registry(template_conf)?),
        snowflakes: snowflakes::IdSnowflakes::new(1)?,
        offload: rt_handle
    };
    let router = routing::MakeRouter {
        state: state.clone()
    };

    let mut futures_list = Vec::new();

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
    router: routing::MakeRouter,
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