use std::sync::{Arc};

use bb8::{Pool};
use tokio_postgres::{NoTls};
use bb8_postgres::{PostgresConnectionManager};

use crate::db::{types};

pub struct DBState {
    pub pool: types::Pool
}

pub type ArcDBState = Arc<DBState>;