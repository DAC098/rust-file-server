use std::sync::Arc;

use tokio_postgres::{NoTls, Config, Error};

use crate::db::types;

pub struct DBState {
    pub pool: types::Pool
}

pub type ArcDBState = Arc<DBState>;

impl DBState {
    pub async fn new(conf: Config) -> Result<ArcDBState, Error> {
        Ok(Arc::new(DBState {
            pool: types::Pool::builder().build(
                types::ConnectionManager::new(conf, NoTls)
            ).await?
        }))
    }
}