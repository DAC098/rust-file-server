use tokio_postgres::Config;
use crate::config::DBConfig;

pub mod types;
// pub mod error;
mod shared_state;
pub mod record;

pub use shared_state::*;

pub fn build_config(conf: DBConfig) -> Config {
    let mut rtn = Config::new();
    rtn.user(conf.username.as_ref());
    rtn.password(conf.password);
    rtn.host(conf.hostname.as_ref());
    rtn.port(conf.port);
    rtn.dbname(conf.database.as_ref());
    rtn
}