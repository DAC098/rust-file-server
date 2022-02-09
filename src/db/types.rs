use bb8::{
    Pool as BB8Pool, 
    PooledConnection as BB8PooledConnection
};
use tokio_postgres::NoTls;
use bb8_postgres::PostgresConnectionManager;

pub type ConnectionManager = PostgresConnectionManager<NoTls>;

pub type Pool = BB8Pool<ConnectionManager>;
pub type PoolConn<'a> = BB8PooledConnection<'a, ConnectionManager>;