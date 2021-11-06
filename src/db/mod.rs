use std::sync::{Arc};

use tokio_postgres::{Config, NoTls, Error};

use crate::config::{DBConfig};

pub mod types;
pub mod shared_state;

pub fn build_config(conf: DBConfig) -> Config {
    let mut rtn = Config::new();
    rtn.user(conf.username.as_ref());
    rtn.password(conf.password);
    rtn.host(conf.hostname.as_ref());
    rtn.port(conf.port);
    rtn.dbname(conf.database.as_ref());
    rtn
}

pub async fn build_shared_state(
    db_conf: Config
) -> std::result::Result<shared_state::ArcDBState, Error> {
    Ok(Arc::new(shared_state::DBState {
        pool: types::Pool::builder().build(
            types::ConnectionManager::new(db_conf, NoTls)
        ).await?
    }))
}