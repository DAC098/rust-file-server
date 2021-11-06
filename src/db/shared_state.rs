use std::sync::{Arc};

use bb8::{Pool};
use tokio_postgres::{NoTls, Config, Error};
use bb8_postgres::{PostgresConnectionManager};

use crate::db::{types};

pub struct DBState {
    pool: types::Pool
}

pub type ArcDBState = Arc<DBState>;

pub async fn build_shared_state(
    db_conf: Config
) -> std::result::Result<ArcDBState, Error> {
    Ok(Arc::new(DBState {
        pool: types::Pool::builder().build(
            types::ConnectionManager::new(db_conf, NoTls)
        ).await?
    }))
}